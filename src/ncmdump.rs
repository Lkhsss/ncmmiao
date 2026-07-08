use crate::apperror::AppError;
use crate::cipher::{self, NEW_KEY_CORE, NEW_KEY_META};
use crate::messager;
use base64::{self, Engine};
use crossterm::style::{Color, Stylize};
use log::{debug, info, trace, warn};
use messager::Signals;
use metaflac::Tag as FlacTag;
use metaflac::block::PictureType;
use serde_json::{self, Value};
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::str::from_utf8;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Debug)]
pub struct Ncmfile {
    pub reader: BufReader<File>,
    pub filename: String,
    pub size: u64,
}

impl Ncmfile {
    pub fn new(filepath: &str) -> Result<Ncmfile, AppError> {
        let file = match File::open(filepath) {
            Ok(f) => f,
            Err(_) => return Err(AppError::FileReadError),
        };
        let size = file
            .metadata()
            .map_err(|_| AppError::CannotReadMetaInfo)?
            .len();
        let filename = match Path::new(filepath).file_stem() {
            Some(f) => f.to_str().ok_or(AppError::FileReadError)?.to_string(),
            None => return Err(AppError::CannotReadFileName),
        };
        Ok(Ncmfile {
            reader: BufReader::with_capacity(64 * 1024, file),
            filename,
            size,
        })
    }

    pub fn seekread(&mut self, length: u64) -> Result<Vec<u8>, AppError> {
        let pos = self
            .reader
            .stream_position()
            .map_err(|_| AppError::FileReadError)?;
        if pos + length > self.size {
            Err(AppError::FileReadError)
        } else {
            let mut buf = vec![0; length as usize];
            self.reader
                .read_exact(&mut buf)
                .map_err(|_| AppError::FileReadError)?;
            Ok(buf)
        }
    }

    pub fn skip(&mut self, length: u64) -> Result<(), AppError> {
        let pos = self
            .reader
            .stream_position()
            .map_err(|_| AppError::FileReadError)?;
        if pos + length > self.size {
            Err(AppError::FileReadError)
        } else {
            self.reader
                .seek(SeekFrom::Current(length as i64))
                .map_err(|_| AppError::FileReadError)?;
            Ok(())
        }
    }

    pub fn is_ncm(data: &[u8]) -> Result<(), AppError> {
        let header = from_utf8(data).map_err(|_| AppError::NotNcmFile)?;
        if header != "CTENFDAM" {
            Err(AppError::NotNcmFile)
        } else {
            Ok(())
        }
    }

    fn get_filename(&self) -> &str {
        &self.filename
    }
}

impl Ncmfile {
    pub fn dump(
        &mut self,
        outputdir: &Path,
        tx: crossbeam_channel::Sender<messager::Message>,
        force_save: bool,
        cancel: Arc<AtomicBool>,
    ) -> Result<(), AppError> {
        let messager = messager::Messager::new(tx);
        let _ = messager.send(Signals::Start);

        trace!("读取magic header");
        let magic_header = self.seekread(8)?;

        trace!("判断是否为ncm格式的文件");
        Self::is_ncm(&magic_header)?;

        trace!("跳过2字节");
        self.skip(2)?;

        trace!("获取RC4密钥长度");
        let key_length = u32::from_le_bytes(
            self.seekread(4)?
                .try_into()
                .map_err(|_| AppError::FileReadError)?,
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
        let _ = messager.send(Signals::GetMetaInfo);

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

        trace!("拼接文件路径");
        let path = {
            let output_filename = format!("{}.{}", self.get_filename(), format);
            debug!("文件名：{}", output_filename.as_str().with(Color::Yellow));
            outputdir.join(output_filename)
        };

        debug!("文件路径: {:?}", path);

        if !force_save && Path::new(&path).exists() {
            return Err(AppError::ProtectFile);
        }

        self.skip(4)?;

        trace!("跳过5个字节");
        self.skip(5)?;

        let _ = messager.send(Signals::GetCover);
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
        let _ = messager.send(Signals::Decrypt);

        let remaining = self.size
            - self
                .reader
                .stream_position()
                .map_err(|_| AppError::FileReadError)?;
        let mut music_data = Vec::with_capacity(remaining as usize);
        let mut chunk = vec![0u8; 0x8000];
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(AppError::Cancelled);
            }
            let n = self
                .reader
                .read(&mut chunk)
                .map_err(|_| AppError::FileReadError)?;
            if n == 0 {
                break;
            }

            for (idx, byte) in chunk[..n].iter_mut().enumerate() {
                *byte ^= decrypt_table[(idx + 1) & 0xFF];
            }
            music_data.extend_from_slice(&chunk[..n]);
        }

        let _ = messager.send(Signals::Save);

        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or(AppError::CannotReadMetaInfo)?;

        // 根据格式分别嵌入封面
        match extension {
            "flac" => {
                let mut cursor = Cursor::new(&music_data[..]);
                let mut tag =
                    FlacTag::read_from(&mut cursor).map_err(|_| AppError::CoverCannotSave)?;
                tag.add_picture(
                    "image/jpeg".to_string(),
                    PictureType::CoverFront,
                    image_data,
                );
                let mut file = File::create(&path).map_err(|_| AppError::FileWriteError)?;
                tag.write_to(&mut file)
                    .map_err(|_| AppError::CoverCannotSave)?;
                file.write_all(&music_data)
                    .map_err(|_| AppError::FileWriteError)?;
            }
            // "mp4" | "m4a" | "m4b" | "m4r" | "m4v" => {
            //     let mut cursor = Cursor::new(&music_data[..]);
            //     let mut tag =
            //         Mp4Tag::read_from(&mut cursor).map_err(|_| AppError::CoverCannotSave)?;
            //     tag.set_artwork(Img::jpeg(image_data));
            //     let mut file = File::create(&path).map_err(|_| AppError::FileWriteError)?;
            //     tag.write_to(&mut file)
            //         .map_err(|_| AppError::CoverCannotSave)?;
            //     file.write_all(&music_data)
            //         .map_err(|_| AppError::FileWriteError)?;
            // }
            _ => {
                let mut file = File::create(&path).map_err(|_| AppError::FileWriteError)?;
                file.write_all(&music_data)
                    .map_err(|_| AppError::FileWriteError)?;
            }
        }

        info!(
            "[{}] 文件已保存到: {}",
            self.get_filename().with(Color::Yellow),
            path.to_str().ok_or(AppError::SaveError)?.with(Color::Cyan)
        );
        if format == "m4a" {
            warn!(
                "[{}] 该文件编码为 AAC 格式，大多数播放器无法播放",
                self.get_filename().with(Color::Yellow)
            );
        }
        info!(
            "[{}]{}",
            self.get_filename().with(Color::Yellow),
            "解密成功".with(Color::Green)
        );
        let _ = messager.send(Signals::End);
        Ok(())
    }
}
