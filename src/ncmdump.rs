use crate::apperror::AppError;
use crate::cipher::{self, NEW_KEY_CORE, NEW_KEY_META};
use crate::{Ncmfile, Signal};
use base64::{self, Engine};
use crossbeam_channel::SendError;
use log::{debug, info, trace, warn};
use metaflac::Tag as FlacTag;
use metaflac::block::PictureType;
use serde_json::{self, Value};
use std::fs::File;
use std::io::{Cursor, Read, Seek, Write};
use std::path::Path;
use std::str::from_utf8;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

struct Messager<T>(Option<crossbeam_channel::Sender<T>>);

impl<T> Messager<T> {
    fn send(&self, msg: T) -> Result<(), SendError<T>> {
        if let Some(sender) = &self.0 {
            sender.send(msg)
        } else {
            Ok(())
        }
    }
}
/// 解密得到的数据：音乐字节、封面字节与音频格式
struct DecryptedData {
    music_data: Vec<u8>,
    cover_data: Vec<u8>,
    format: String,
}

/// 解析出的 meta 信息：原始 JSON 字符串与结构化对象
struct MetaInfo {
    raw: String,
    value: Value,
}

impl Ncmfile {
    /// 读取 4 字节小端无符号整数（文件损坏或不足 4 字节时报错）
    fn read_u32(&mut self) -> Result<u32, AppError> {
        let buf = self.seekread(4)?;
        Ok(u32::from_le_bytes(
            buf.try_into().map_err(|_| AppError::FileDataError)?,
        ))
    }

    /// 将游标定位到 RC4 密钥长度字段（seek 回文件头并跳过 magic 与 2 字节 gap）
    fn seek_to_key(&mut self) -> Result<(), AppError> {
        self.seek(std::io::SeekFrom::Start(0))?;
        self.skip(8)?;
        self.skip(2)?;
        Ok(())
    }

    /// 读取并解析 RC4 密钥（调用后游标位于 meta_length 字段）
    fn read_key(&mut self) -> Result<Vec<u8>, AppError> {
        self.seek_to_key()?;
        let key_length = self.read_u32()? as u64;
        let mut key_data = self.seekread(key_length)?;
        cipher::parse_key(&mut key_data[..]);
        let key_data = cipher::aes128_to_slice(&NEW_KEY_CORE, &key_data)?;
        let mut key_data = cipher::unpad(&key_data[..]);
        if key_data.len() < 17 {
            return Err(AppError::FileDataError);
        }
        key_data.drain(..17);
        Ok(key_data)
    }

    /// 跳过 RC4 密钥（调用后游标位于 meta_length 字段）
    fn skip_key(&mut self) -> Result<(), AppError> {
        self.seek_to_key()?;
        let key_length = self.read_u32()? as u64;
        self.skip(key_length)?;
        Ok(())
    }

    /// 跳过 meta 数据及其后的固定 4+5 字节（调用后游标位于 image_length 字段）
    fn skip_meta(&mut self) -> Result<(), AppError> {
        let meta_length = self.read_u32()? as u64;
        self.skip(meta_length)?;
        // meta 数据后为固定区域：4 字节 + 5 字节保留，跳到图片长度字段
        self.skip(4)?;
        self.skip(5)?;
        Ok(())
    }

    /// 读取 meta 数据并解密为 JSON（调用后游标位于 image_length 字段）
    fn read_meta(&mut self) -> Result<MetaInfo, AppError> {
        let meta_length = self.read_u32()? as u64;
        let mut meta_data = self.seekread(meta_length)?;

        for item in &mut meta_data {
            *item ^= 0x63;
        }
        // meta 数据前 22 字节为固定信息（"music" 标识与长度等），跳过
        let base64_data = meta_data
            .get(22..)
            .ok_or(AppError::CannotReadMetaInfo)?;
        let mut decode_data = Vec::<u8>::new();
        if base64::engine::general_purpose::STANDARD
            .decode_vec(base64_data, &mut decode_data)
            .is_err()
        {
            return Err(AppError::CannotReadMetaInfo);
        }
        let aes_data = cipher::aes128_to_slice(&NEW_KEY_META, &decode_data)?;
        let unpadded = cipher::unpad(&aes_data);
        let raw = match from_utf8(unpadded.get(6..).ok_or(AppError::CannotReadMetaInfo)?) {
            Ok(o) => o.to_owned(),
            Err(_) => return Err(AppError::CannotReadMetaInfo),
        };
        debug!("json_data: {}", raw);
        let value: Value = match serde_json::from_str(&raw) {
            Ok(o) => o,
            Err(_) => return Err(AppError::CannotReadMetaInfo),
        };
        debug!("{}", value);

        // meta 数据后为固定区域：4 字节 + 5 字节保留，跳到图片长度字段
        self.skip(4)?;
        self.skip(5)?;
        Ok(MetaInfo { raw, value })
    }

    /// 读取封面数据（调用后游标位于音乐数据处）
    fn read_cover(&mut self) -> Result<Vec<u8>, AppError> {
        let image_length = self.read_u32()? as u64;
        self.seekread(image_length)
    }

    /// 解密剩余音乐数据，并按字节发送进度信号（0.0~1.0）
    fn decrypt_music(
        &mut self,
        key_data: &[u8],
        sender: &Messager<Signal>,
        cancel: &AtomicBool,
    ) -> Result<Vec<u8>, AppError> {
        let decrypt_table = cipher::build_decrypt_table(key_data);

        let _ = sender.send(Signal::Decrypt(0.0));

        let remaining = self
            .size
            .saturating_sub(
                self.reader
                    .stream_position()
                    .map_err(AppError::FileReadError)?,
            );
        // 按字节统计进度：progress = 已解密字节数 / 音乐数据总字节数（0.0~1.0）
        let total = remaining.max(1);
        let mut processed: u64 = 0;
        let mut last_percent: u8 = 0;
        let mut music_data = Vec::with_capacity(remaining as usize);
        let mut chunk = vec![0u8; 0x8000];
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(AppError::Cancelled);
            }
            let n = self
                .reader
                .read(&mut chunk)
                .map_err(AppError::FileReadError)?;
            if n == 0 {
                break;
            }

            for (idx, byte) in chunk[..n].iter_mut().enumerate() {
                *byte ^= decrypt_table[(idx + 1) & 0xFF];
            }
            music_data.extend_from_slice(&chunk[..n]);
            processed += n as u64;

            // 仅在整百分比变化时发送进度，避免大文件产生过多消息
            let percent = (processed * 100 / total) as u8;
            if percent != last_percent {
                last_percent = percent;
                let _ = sender.send(Signal::Decrypt(percent as f64 / 100.0));
            }
        }
        let _ = sender.send(Signal::Decrypt(1.0));
        Ok(music_data)
    }

    /// 读取音乐文件头信息，返回原始 JSON 字符串
    /// 会自动移动游标
    pub fn get_music_info(&mut self) -> Result<String, AppError> {
        trace!("读取音乐信息");
        self.skip_key()?;
        let meta = self.read_meta()?;
        Ok(meta.raw)
    }
    /// 获取图片数据
    /// 会自动移动游标
    pub fn get_pic(&mut self) -> Result<Vec<u8>, AppError> {
        trace!("获取图片数据");
        self.skip_key()?;
        self.skip_meta()?;
        self.read_cover()
    }
    /// 获取解密后的音乐数据，需配合音乐信息中的格式来知晓文件后缀
    /// 会自动移动游标
    pub fn get_music_data(
        &mut self,
        tx: Option<crossbeam_channel::Sender<Signal>>,
        cancel: Arc<AtomicBool>,
    ) -> Result<Vec<u8>, AppError> {
        let sender = Messager(tx);
        let _ = sender.send(Signal::Start);
        let key_data = self.read_key()?;
        let _ = sender.send(Signal::GetMetaInfo);
        self.skip_meta()?;
        let _ = sender.send(Signal::GetCover);
        let _ = self.read_cover()?;
        self.decrypt_music(&key_data, &sender, cancel.as_ref())
    }

    /// 解密但不保存文件，直接返回解密后的音乐字节。
    pub fn dump(
        &mut self,
        tx: Option<crossbeam_channel::Sender<Signal>>,
        cancel: Arc<AtomicBool>,
    ) -> Result<Vec<u8>, AppError> {
        let sender = Messager(tx);
        let data = self.parse_and_decrypt(&sender, cancel.as_ref())?;
        Ok(data.music_data)
    }

    /// 解析 ncm 文件并解密音乐数据（含封面与格式），期间发送进度信号。
    fn parse_and_decrypt(
        &mut self,
        sender: &Messager<Signal>,
        cancel: &AtomicBool,
    ) -> Result<DecryptedData, AppError> {
        let _ = sender.send(Signal::Start);
        // 文件头（magic）校验已在 Ncmfile::new 中完成
        let key_data = self.read_key()?;
        let _ = sender.send(Signal::GetMetaInfo);

        let meta = self.read_meta()?;
        let format = meta
            .value
            .get("format")
            .and_then(|v| v.as_str())
            .ok_or(AppError::CannotReadMetaInfo)?
            .to_string();
        let _ = sender.send(Signal::GetCover);
        let cover_data = self.read_cover()?;

        let music_data = self.decrypt_music(&key_data, sender, cancel)?;

        Ok(DecryptedData {
            music_data,
            cover_data,
            format,
        })
    }

    /// 全自动的解密函数
    pub fn dump_to_file(
        &mut self,
        outputdir: &Path,
        tx: Option<crossbeam_channel::Sender<Signal>>,
        force_save: bool,
        cancel: Arc<AtomicBool>,
    ) -> Result<(), AppError> {
        let sender = Messager(tx);
        let data = self.parse_and_decrypt(&sender, cancel.as_ref())?;
        let format = &data.format;

        trace!("拼接文件路径");
        let path = {
            let output_filename = format!("{}.{}", self.get_filename(), format);
            debug!("文件名：{}", output_filename.as_str());
            outputdir.join(output_filename)
        };
        debug!("文件路径: {:?}", path);

        if !force_save && Path::new(&path).exists() {
            return Err(AppError::ProtectFile);
        }

        let _ = sender.send(Signal::Save);

        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or(AppError::CannotReadMetaInfo)?;

        // 根据格式分别嵌入封面
        match extension {
            "flac" => {
                let mut cursor = Cursor::new(&data.music_data[..]);
                let mut tag =
                    FlacTag::read_from(&mut cursor).map_err(|_| AppError::CoverCannotSave)?;
                tag.add_picture(
                    "image/jpeg".to_string(),
                    PictureType::CoverFront,
                    data.cover_data,
                );
                let mut file = File::create(&path).map_err(|_| AppError::FileWriteError)?;
                tag.write_to(&mut file)
                    .map_err(|_| AppError::CoverCannotSave)?;
                file.write_all(&data.music_data)
                    .map_err(|_| AppError::FileWriteError)?;
            }
            // "mp4" | "m4a" | "m4b" | "m4r" | "m4v" => {
            //     let mut cursor = Cursor::new(&data.music_data[..]);
            //     let mut tag =
            //         Mp4Tag::read_from(&mut cursor).map_err(|_| AppError::CoverCannotSave)?;
            //     tag.set_artwork(Img::jpeg(data.cover_data));
            //     let mut file = File::create(&path).map_err(|_| AppError::FileWriteError)?;
            //     tag.write_to(&mut file)
            //         .map_err(|_| AppError::CoverCannotSave)?;
            //     file.write_all(&data.music_data)
            //         .map_err(|_| AppError::FileWriteError)?;
            // }
            _ => {
                let mut file = File::create(&path).map_err(|_| AppError::FileWriteError)?;
                file.write_all(&data.music_data)
                    .map_err(|_| AppError::FileWriteError)?;
            }
        }

        info!(
            "[{}] 文件已保存到: {}",
            self.get_filename(),
            path.to_str().ok_or(AppError::SaveError)?
        );
        if format.as_str() == "m4a" {
            warn!(
                "[{}] 该文件编码为 AAC 格式，大多数播放器无法播放",
                self.get_filename()
            );
        }
        info!("[{}]解密成功", self.get_filename());
        let _ = sender.send(Signal::End);
        Ok(())
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_magic_header() {
        assert!(Ncmfile::is_ncm(b"CTENFDAM").is_ok());
        assert!(matches!(
            Ncmfile::is_ncm(b"NOTHEAD!"),
            Err(AppError::NotNcmFile)
        ));
        assert!(matches!(Ncmfile::is_ncm(&[]), Err(AppError::NotNcmFile)));
    }

    #[test]
    fn dump_reports_byte_progress() {
        use crossbeam_channel::unbounded;

        let path = format!(
            "{}/test/1/$uNDOWN - Stray.wav.ncm",
            env!("CARGO_MANIFEST_DIR")
        );
        let mut ncm = Ncmfile::new(&path).expect("测试 ncm 文件应当可以打开");

        let (tx, rx) = unbounded::<Signal>();
        let cancel = Arc::new(AtomicBool::new(false));
        let bytes = ncm.dump(Some(tx), cancel).expect("解密应当成功");
        assert!(!bytes.is_empty(), "解密结果不应为空");

        // dump 不保存文件，只发送解析/解密相关的进度信号（无 Save/End）
        let mut saw_start = false;
        let mut progress = Vec::new();
        for s in rx.try_iter() {
            match s {
                Signal::Start => saw_start = true,
                Signal::Decrypt(p) => progress.push(p),
                _ => {}
            }
        }

        assert!(saw_start, "应当收到 Start 信号");
        assert!(!progress.is_empty(), "应当收到至少一条进度信号");
        assert_eq!(progress.first(), Some(&0.0), "进度应从 0.0 开始");
        assert_eq!(progress.last(), Some(&1.0), "进度应以 1.0 结束");
        for pair in progress.windows(2) {
            assert!(pair[1] >= pair[0], "进度应当单调不减: {progress:?}");
        }
        assert!(
            progress.iter().all(|&p| (0.0..=1.0).contains(&p)),
            "进度值应在 0.0~1.0 之间: {progress:?}"
        );
    }

    #[test]
    fn dump_to_file_saves_output() {
        use crossbeam_channel::unbounded;

        let path = format!(
            "{}/test/1/$uNDOWN - Stray.wav.ncm",
            env!("CARGO_MANIFEST_DIR")
        );
        let mut ncm = Ncmfile::new(&path).expect("测试 ncm 文件应当可以打开");

        let (tx, _rx) = unbounded::<Signal>();
        let cancel = Arc::new(AtomicBool::new(false));
        let outdir = std::env::temp_dir().join("ncmmiao_dump_to_file_test");
        std::fs::create_dir_all(&outdir).expect("创建临时输出目录");
        ncm.dump_to_file(&outdir, Some(tx), true, cancel)
            .expect("解密保存应当成功");

        // 输出目录下应生成解密后的文件
        let files = std::fs::read_dir(&outdir)
            .expect("读取输出目录")
            .filter_map(Result::ok)
            .count();
        assert!(files > 0, "输出目录应当包含解密文件");
    }

    #[test]
    fn get_pic_and_get_music_data_consistent_with_parse_and_decrypt() {
        use crossbeam_channel::unbounded;

        let path = format!(
            "{}/test/1/$uNDOWN - Stray.wav.ncm",
            env!("CARGO_MANIFEST_DIR")
        );

        // 基准：parse_and_decrypt 的结果（已验证正确）
        let mut ncm = Ncmfile::new(&path).expect("测试 ncm 文件应当可以打开");
        let (tx, _rx) = unbounded::<Signal>();
        let cancel = Arc::new(AtomicBool::new(false));
        let data = ncm
            .parse_and_decrypt(&Messager(Some(tx)), cancel.as_ref())
            .expect("解析与解密应当成功");

        // get_pic 独立解析应与 parse_and_decrypt 的封面一致
        let mut ncm = Ncmfile::new(&path).expect("测试 ncm 文件应当可以打开");
        let pic = ncm.get_pic().expect("获取封面应当成功");
        assert_eq!(pic, data.cover_data, "封面数据应一致");

        // get_music_data 独立解析应与 parse_and_decrypt 的音乐一致
        let mut ncm = Ncmfile::new(&path).expect("测试 ncm 文件应当可以打开");
        let (tx, _rx) = unbounded::<Signal>();
        let cancel = Arc::new(AtomicBool::new(false));
        let music = ncm
            .get_music_data(Some(tx), cancel)
            .expect("获取音乐应当成功");
        assert_eq!(music, data.music_data, "音乐数据应一致");
    }

    #[test]
    fn get_music_info_returns_original_json() {
        let path = format!(
            "{}/test/1/$uNDOWN - Stray.wav.ncm",
            env!("CARGO_MANIFEST_DIR")
        );
        let mut ncm = Ncmfile::new(&path).expect("测试 ncm 文件应当可以打开");
        let info = ncm.get_music_info().expect("获取音乐信息应当成功");
        let value: serde_json::Value =
            serde_json::from_str(&info).expect("返回的 JSON 应可重新解析");
        assert!(value.get("format").is_some(), "应包含 format 字段");
    }

    #[test]
    fn seekread_short_read_and_skip_past_eof() {
        use std::io::SeekFrom;

        let path = format!(
            "{}/test/1/$uNDOWN - Stray.wav.ncm",
            env!("CARGO_MANIFEST_DIR")
        );
        let mut ncm = Ncmfile::new(&path).expect("测试 ncm 文件应当可以打开");
        let size = ncm.size;

        // 越过文件末尾后读取不应 panic，应返回空
        ncm.skip(size + 100).expect("越过 EOF 的 skip 应成功");
        assert!(ncm.seekread(10).expect("读取应成功").is_empty());

        // 恰好剩余指定长度时应读到完整数据（回归 rest_len == length 分支）
        ncm.seek(SeekFrom::Start(size - 4))
            .expect("seek 应成功");
        assert_eq!(ncm.seekread(4).expect("读取应成功").len(), 4);

        // 请求长度超过剩余字节时应短读
        ncm.seek(SeekFrom::Start(size - 4))
            .expect("seek 应成功");
        assert_eq!(ncm.seekread(100).expect("读取应成功").len(), 4);
    }
}
