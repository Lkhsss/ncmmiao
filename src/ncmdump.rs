use crate::apperror::AppError;
use crate::messager;
use aes::Aes128;
use base64::{self, Engine};
use cipher::{Array, BlockCipherDecrypt, KeyInit};
use crossterm::style::{Color, Stylize};
use generic_array::typenum::U16;
use log::{debug, info, trace};
use messager::Signals;
use metaflac::Tag as FlacTag;
use metaflac::block::PictureType;
use serde_json::{self, Value};
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::str::from_utf8;

const NEW_KEY_CORE: [u8; 16] = [
    0x68, 0x7A, 0x48, 0x52, 0x41, 0x6D, 0x73, 0x6F, 0x35, 0x6B, 0x49, 0x6E, 0x62, 0x61, 0x78, 0x57,
];
const NEW_KEY_META: [u8; 16] = [
    0x23, 0x31, 0x34, 0x6C, 0x6A, 0x6B, 0x5F, 0x21, 0x5C, 0x5D, 0x26, 0x30, 0x55, 0x3C, 0x27, 0x28,
];
#[derive(Debug)]
pub struct Ncmfile {
    pub reader: BufReader<File>,
    pub filename: String,
    pub size: u64,
    pub position: u64,
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
            position: 0,
        })
    }

    pub fn seekread(&mut self, length: u64) -> Result<Vec<u8>, AppError> {
        if self.position + length > self.size {
            Err(AppError::FileReadError)
        } else {
            let mut buf = vec![0; length as usize];
            self.reader
                .read_exact(&mut buf)
                .map_err(|_| AppError::FileReadError)?;
            self.position += length;
            Ok(buf)
        }
    }

    pub fn skip(&mut self, length: u64) -> Result<(), AppError> {
        if self.position + length > self.size {
            Err(AppError::FileReadError)
        } else {
            self.reader
                .seek(SeekFrom::Current(length as i64))
                .map_err(|_| AppError::FileReadError)?;
            self.position += length;
            Ok(())
        }
    }

    fn parse_key(key: &mut [u8]) {
        for item in &mut *key {
            *item ^= 0x64;
        }
    }

    fn is_ncm(data: &[u8]) -> Result<(), AppError> {
        let header = from_utf8(data).map_err(|_| AppError::NotNcmFile)?;
        if header != "CTENFDAM" {
            Err(AppError::NotNcmFile)
        } else {
            Ok(())
        }
    }

    fn unpad(data: &[u8]) -> Vec<u8> {
        data[..data.len() - data[data.len() - 1] as usize].to_vec()
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
        Self::parse_key(&mut key_data[..]);
        let key_data = aes128_to_slice(&NEW_KEY_CORE, key_data)?;
        let mut key_data = Self::unpad(&key_data[..]);
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
            let aes_data = aes128_to_slice(&NEW_KEY_META, decode_data)?;
            let unpadded = Self::unpad(&aes_data);
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

        trace!("拼接文件路径");
        let path = {
            let output_filename = format!(
                "{}.{}",
                self.get_filename(),
                meta_data
                    .get("format")
                    .ok_or(AppError::CannotReadMetaInfo)?
                    .as_str()
                    .ok_or(AppError::CannotReadMetaInfo)?
            );
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
        let decrypt_table = {
            let key_length = key_data.len();
            let mut key_box = (0..=255).collect::<Vec<u8>>();
            let mut last_byte = 0u64;
            let mut temp;
            let mut key_offset = 0;

            for i in 0..=255 {
                let swap = key_box[i] as u64;
                temp = (swap + last_byte + key_data[key_offset] as u64) & 0xFF;
                key_offset += 1;
                if key_offset >= key_length {
                    key_offset = 0;
                }
                key_box[i] = key_box[temp as usize];
                key_box[temp as usize] = swap as u8;
                last_byte = temp;
            }

            let mut table = [0u8; 256];
            for j in 0..256usize {
                table[j] = key_box[(key_box[j] as usize
                    + key_box[(key_box[j] as usize + j) & 0xFF] as usize)
                    & 0xFF];
            }
            table
        };

        trace!("解密音乐数据");
        let _ = messager.send(Signals::Decrypt);

        let mut music_data = Vec::with_capacity((self.size - self.position) as usize);
        let mut music_data = Vec::with_capacity((self.size - self.position) as usize);
        let mut chunk = vec![0u8; 0x8000];
        loop {
            let n = self
                .reader
                .read(&mut chunk)
                .map_err(|_| AppError::FileReadError)?;
            if n == 0 {
                break;
            }
            self.position += n as u64;

            for (idx, byte) in chunk[..n].iter_mut().enumerate() {
                *byte ^= decrypt_table[(idx + 1) & 0xFF];
            }
            music_data.extend_from_slice(&chunk[..n]);
        }

        let _ = messager.send(Signals::Save);

        // 从内存 Cursor 解析 FLAC，嵌入封面，一次写出
        let mut cursor = Cursor::new(&music_data[..]);
        let mut tag = FlacTag::read_from(&mut cursor).map_err(|_| AppError::CoverCannotSave)?;
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

        info!(
            "[{}] 文件已保存到: {}",
            self.get_filename().with(Color::Yellow),
            path.to_str().ok_or(AppError::SaveError)?.with(Color::Cyan)
        );
        info!(
            "[{}]{}",
            self.get_filename().with(Color::Yellow),
            "解密成功".with(Color::Green)
        );
        let _ = messager.send(Signals::End);
        Ok(())
    }
}

fn convert_to_arrays(input: &[u8]) -> Result<Vec<Array<u8, U16>>, AppError> {
    if input.len() % 16 != 0 {
        return Err(AppError::FileDataError);
    }

    Ok(input
        .chunks(16)
        .map(|chunk| <Array<u8, U16>>::try_from(chunk).map_err(|_| AppError::FileDataError))
        .collect::<Result<Vec<_>, _>>()?)
}

fn aes128_to_slice<T: AsRef<[u8]>>(key: &T, blocks: Vec<u8>) -> Result<Vec<u8>, AppError> {
    trace!("进行AES128解密");
    let key: &Array<u8, U16> = key
        .as_ref()
        .try_into()
        .map_err(|_| AppError::FileDataError)?;

    let mut blocks = convert_to_arrays(&blocks)?;

    let cipher = Aes128::new(key);

    cipher.decrypt_blocks(&mut blocks);

    let mut x: Vec<u8> = Vec::with_capacity(blocks.len() * 16);
    for block in blocks.iter() {
        x.extend_from_slice(block);
    }
    Ok(x)
}
