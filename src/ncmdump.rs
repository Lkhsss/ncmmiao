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

impl Ncmfile {
    pub fn get_music_info(&mut self) {}
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

        trace!("读取magic header");
        let magic_header = self.seekread(8)?;

        // trace!("判断是否为ncm格式的文件");
        // Self::is_ncm(&magic_header)?;
        // 文件头校验提前到new函数中

        trace!("跳过2字节");
        self.skip(2)?;

        trace!("获取RC4密钥长度");
        let key_length = u32::from_le_bytes(
            self.seekread(4)?
                .try_into()
                .map_err(|_| AppError::NumParseError)?,
        ) as u64;

        trace!("读取RC4密钥");
        let mut key_data = self.seekread(key_length)?;
        cipher::parse_key(&mut key_data[..]);
        let key_data = cipher::aes128_to_slice(&NEW_KEY_CORE, &key_data)?;
        let mut key_data = cipher::unpad(&key_data[..]);
        key_data.drain(..17);

        trace!("获取meta信息数据大小");
        let meta_length = u32::from_le_bytes(
            self.seekread(4)?
                .try_into()
                .map_err(|_| AppError::FileDataError)?,
        ) as u64;
        let _ = sender.send(Signal::GetMetaInfo);

        trace!("读取meta信息");
        let meta_data = {
            let mut meta_data = self.seekread(meta_length)?;

            for item in &mut meta_data {
                *item ^= 0x63;
            }
            let mut decode_data = Vec::<u8>::new();
            if base64::engine::general_purpose::STANDARD
                .decode_vec(&mut meta_data[22..], &mut decode_data)
                .is_err()
            {
                return Err(AppError::CannotReadMetaInfo);
            };
            let aes_data = cipher::aes128_to_slice(&NEW_KEY_META, &decode_data)?;
            let unpadded = cipher::unpad(&aes_data);
            let json_data = match from_utf8(&unpadded[6..]) {
                Ok(o) => o.to_owned(),
                Err(_) => return Err(AppError::CannotReadMetaInfo),
            };
            debug!("json_data: {}", json_data);
            let data: Value = match serde_json::from_str(&json_data[..]) {
                Ok(o) => o,
                Err(_) => return Err(AppError::CannotReadMetaInfo),
            };
            data
        };
        debug!("{}", meta_data);

        let format = meta_data
            .get("format")
            .ok_or(AppError::CannotReadMetaInfo)?
            .as_str()
            .ok_or(AppError::CannotReadMetaInfo)?;

        self.skip(4)?;

        trace!("跳过5个字节");
        self.skip(5)?;

        let _ = sender.send(Signal::GetCover);
        trace!("获取图片数据的大小");
        let image_data_length = u32::from_le_bytes(
            self.seekread(4)?
                .try_into()
                .map_err(|_| AppError::FileDataError)?,
        ) as u64;

        let image_data = self.seekread(image_data_length)?;

        trace!("组成密码盒");
        let decrypt_table = cipher::build_decrypt_table(&key_data);

        trace!("解密音乐数据");
        let _ = sender.send(Signal::Decrypt(0.0));

        let remaining = self.size
            - self
                .reader
                .stream_position()
                .map_err(AppError::FileReadError)?;
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

        Ok(DecryptedData {
            music_data,
            cover_data: image_data,
            format: format.to_string(),
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

impl Ncmfile {}

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
}
