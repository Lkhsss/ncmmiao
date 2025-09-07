use crate::apperror::AppError;
use crate::messager;
use aes::cipher::generic_array::typenum::U16;
use aes::cipher::{generic_array::GenericArray, BlockDecrypt, KeyInit};
use aes::Aes128;
use audiotags::{MimeType, Picture, Tag};
use base64::{self, Engine};
use crossterm::style::{Color, Stylize}; //防止windows终端乱码
use log::{debug, info, trace};
use messager::Signals;
use serde_derive::{Deserialize, Serialize};
use serde_json::{self, Value};
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::str::from_utf8;
use std::vec;

// lazy_static! {
//     // 解密需要的密钥
//     static ref KEY_CORE: Vec<u8> = decode("687A4852416D736F356B496E62617857").unwrap();//绝对正确
//     static ref KEY_META: Vec<u8> = decode("2331346C6A6B5F215C5D2630553C2728").unwrap();
// }

// 原KEY_CORE数据：687A4852416D736F356B496E62617857
const NEW_KEY_CORE: [u8; 16] = [
    0x68, 0x7A, 0x48, 0x52, 0x41, 0x6D, 0x73, 0x6F, 0x35, 0x6B, 0x49, 0x6E, 0x62, 0x61, 0x78, 0x57,
];
// 原KEY_META数据：2331346C6A6B5F215C5D2630553C2728
const NEW_KEY_META: [u8; 16] = [
    0x23, 0x31, 0x34, 0x6C, 0x6A, 0x6B, 0x5F, 0x21, 0x5C, 0x5D, 0x26, 0x30, 0x55, 0x3C, 0x27, 0x28,
];
#[derive(Debug)]
#[allow(unused_variables)]
pub struct Ncmfile {
    /// 文件对象
    pub file: File,
    /// 文件名称，不带文件后缀
    pub filename: String,
    /// 文件名称，带后缀名
    pub fullfilename: String,
    /// 文件大小
    pub size: u64,
    /// 游标
    pub position: u64,
}

impl Ncmfile {
    /// 各种工具方法
    pub fn new(filepath: &str) -> Result<Ncmfile, AppError> {
        let file = match File::open(filepath) {
            Ok(f) => f,
            Err(_) => return Err(AppError::FileReadError),
        };
        let path = Path::new(filepath);
        let fullfilename = path
            .file_name()
            .ok_or(AppError::FileReadError)?
            .to_str()
            .ok_or(AppError::FileReadError)?
            .to_string();
        let size = file
            .metadata()
            .map_err(|_| AppError::CannotReadMetaInfo)?
            .len();
        let filename = match Path::new(&filepath).file_stem() {
            Some(f) => f.to_str().ok_or(AppError::FileReadError)?.to_string(),
            None => return Err(AppError::CannotReadFileName),
        };
        Ok(Ncmfile {
            file,
            filename,
            fullfilename,
            size,
            position: 0,
        })
    }
    /// 根据传入的长度来读取文件
    ///
    /// 该函数可以记录上次读取的位置，下次读取时从上次读取的位置开始
    /// - length 想要读取的长度
    pub fn seekread(&mut self, length: u64) -> Result<Vec<u8>, AppError> {
        if self.position + length > self.size {
            Err(AppError::FileReadError)
        } else {
            let mut reader = BufReader::new(&self.file);
            let _ = reader.seek(SeekFrom::Start(self.position));
            let mut buf = vec![0; length as usize];
            let _ = reader.read_exact(&mut buf);
            self.position += length;
            Ok(buf[..].to_vec())
        }
    }

    /// 从指定位置开始读取。
    ///
    /// ！！！该函数仍然会更新游标
    ///
    /// - offset 开始位置
    /// - length 想要读取的长度
    #[allow(dead_code)]
    pub fn seekread_from(&mut self, offset: u64, length: u64) -> Result<Vec<u8>, AppError> {
        if self.position + length > self.size {
            Err(AppError::FileReadError)
        } else {
            let mut reader = BufReader::new(&self.file);
            let _ = reader.seek(SeekFrom::Start(offset));
            let mut buf = vec![0; length as usize];
            let _ = reader.read_exact(&mut buf);
            self.position = offset + length;
            Ok(buf[..].to_vec())
        }
    }
    #[allow(dead_code)]
    pub fn seekread_to_end(&mut self) -> Result<Vec<u8>, std::io::Error> {
        let mut reader = BufReader::new(&self.file);
        reader.seek(SeekFrom::Start(self.position))?;
        let mut buf = vec![0; self.size as usize - self.position as usize];
        reader.read_exact(&mut buf)?;
        self.position = self.size;
        Ok(buf[..].to_vec())
    }
    pub fn seekread_no_error(&mut self, length: u64) -> Vec<u8> {
        if self.position + length > self.size {
            if self.position >= self.size {
                vec![]
            } else {
                let mut reader = BufReader::new(&self.file);
                let _ = reader.seek(SeekFrom::Start(self.position));

                let mut buf: Vec<u8> = vec![0; (self.size - self.position) as usize];
                let _ = reader.read_exact(&mut buf);
                self.position += length;
                buf[..].to_vec()
            }
        } else {
            let mut reader = BufReader::new(&self.file);
            let _ = reader.seek(SeekFrom::Start(self.position));
            let mut buf = vec![0; length as usize];
            let _ = reader.read_exact(&mut buf);
            self.position += length;
            buf[..].to_vec()
        }
    }
    /// 跳过某些数据
    pub fn skip(&mut self, length: u64) -> Result<(), AppError> {
        if self.position + length > self.size {
            Err(AppError::FileReadError)
        } else {
            self.position += length;
            Ok(())
        }
    }
    ///按字节进行0x64异或。
    fn parse_key(key: &mut [u8]) -> &[u8] {
        for item in &mut *key {
            *item ^= 0x64;
        }
        key
    }

    fn save(&mut self, path: &PathBuf, data: Vec<u8>) -> Result<(), AppError> {
        let music_file = match File::create(path) {
            Ok(o) => o,
            Err(_) => return Err(AppError::FileWriteError),
        };
        let mut writer = BufWriter::new(music_file);
        let _ = writer.write_all(&data);
        // 关闭文件
        match writer.flush() {
            Ok(o) => o,
            Err(_) => return Err(AppError::FileWriteError),
        };
        Ok(())
    }
    fn is_ncm(data: Vec<u8>) -> Result<(), AppError> {
        let header = from_utf8(&data).map_err(|_| AppError::NotNcmFile)?;
        if header != "CTENFDAM" {
            Err(AppError::NotNcmFile)
        } else {
            Ok(())
        }
    }
    /// 使用PKCS5Padding标准，去掉填充信息
    fn unpad(data: &[u8]) -> Vec<u8> {
        data[..data.len() - data[data.len() - 1] as usize].to_vec()
    }

    fn get_filename(&self) -> &str {
        &self.filename
    }

    #[allow(dead_code)]
    fn get_fullfilename(&self) -> &str {
        &self.fullfilename
    }
}

impl Ncmfile {
    /// 解密函数
    #[allow(unused_assignments)]
    pub fn dump(
        &mut self,
        outputdir: &Path,
        tx: crossbeam_channel::Sender<messager::Message>,
        force_save: bool,
    ) -> Result<(), AppError> {
        let messager = messager::Messager::new(self.fullfilename.clone(), tx);
        let _ = messager.send(Signals::Start);

        // 获取magic header   应为CTENFDAM
        trace!("读取magic header");
        let magic_header = self.seekread(8)?;

        // 判断是否为ncm格式的文件
        trace!("判断是否为ncm格式的文件");
        Self::is_ncm(magic_header)?;

        // 跳过2字节
        trace!("跳过2字节");
        self.skip(2)?;

        trace!("获取RC4密钥长度");
        //小端模式读取RC4密钥长度 正常情况下应为128
        let key_length = u32::from_le_bytes(
            self.seekread(4)?
                .try_into()
                .map_err(|_| AppError::FileReadError)?,
        ) as u64; //数据长度不够只能使用u32 然后转化为u64
                  // debug!("RC4密钥长度为：{}", key_length);

        //读取密钥 开头应为 neteasecloudmusic
        trace!("读取RC4密钥");
        let mut key_data = self.seekread(key_length)?;
        //aes128解密
        let key_data =
            &aes128_to_slice(&NEW_KEY_CORE, Self::parse_key(&mut key_data[..]).to_vec())?; //先把密钥按照字节进行0x64异或
                                                                                           // RC4密钥
        let key_data = Self::unpad(&key_data[..])[17..].to_vec(); //去掉neteasecloudmusic

        //读取meta信息的数据大小
        trace!("获取meta信息数据大小");
        let meta_length = u32::from_le_bytes(
            self.seekread(4)?
                .try_into()
                .map_err(|_| AppError::FileDataError)?,
        ) as u64;
        let _ = messager.send(Signals::GetMetaInfo);

        // 读取meta信息
        trace!("读取meta信息");
        let meta_data = {
            let mut meta_data = self.seekread(meta_length)?; //读取源数据
                                                             //字节对0x63进行异或。
            for item in &mut meta_data {
                *item ^= 0x63;
            }
            // base64解密
            let mut decode_data = Vec::<u8>::new();
            if base64::engine::general_purpose::STANDARD
                .decode_vec(&mut meta_data[22..], &mut decode_data)
                .is_err()
            {
                return Err(AppError::CannotReadMetaInfo);
            };
            // aes128解密
            let aes_data = aes128_to_slice(&NEW_KEY_META, decode_data)?;
            // unpadding
            let json_data = match String::from_utf8(Self::unpad(&aes_data)[6..].to_vec()) {
                Ok(o) => o,
                Err(_) => return Err(AppError::CannotReadMetaInfo),
            };
            debug!("json_data: {}", json_data);
            let data: Value = match serde_json::from_str(&json_data[..]) {
                Ok(o) => o,
                Err(_) => return Err(AppError::CannotReadMetaInfo),
            }; //解析json数据
            data
        };

        //处理文件路径
        trace!("拼接文件路径");
        let path = {
            let output_filename = &format!(
                "{}.{}",
                self.get_filename(),
                meta_data
                    .get("format")
                    .ok_or(AppError::CannotReadMetaInfo)?
                    .as_str()
                    .ok_or(AppError::CannotReadMetaInfo)?
            )[..];

            // let output_filename = standardize_filename(output_filename);
            debug!("文件名：{}", output_filename.with(Color::Yellow));

            //已在程序开头创建，无需浪费性能
            //链级创建输出目录
            // if fs::create_dir_all(outputdir).is_err() {
            //     return Err(AppError::FileWriteError);
            // }
            outputdir.join(output_filename)
        };

        debug!("文件路径: {:?}", path);

        // 先检查是否存在
        if !force_save && Path::new(&path).exists() {
            return Err(AppError::ProtectFile);
        }

        // 跳过4个字节的校验码
        // trace!("读取校验码");
        // let _crc32 = u32::from_le_bytes(self.seekread(4)?.try_into().map_err(AppError::FileDataError)?) as u64;

        self.skip(4)?;

        // 跳过5个字节
        trace!("跳过5个字节");
        self.skip(5)?;

        let _ = messager.send(Signals::GetCover);
        // 获取图片数据的大小
        trace!("获取图片数据的大小");
        let image_data_length = u32::from_le_bytes(
            self.seekread(4)?
                .try_into()
                .map_err(|_| AppError::FileDataError)?,
        ) as u64;

        // 读取图片，并写入文件当中
        let image_data = self.seekread(image_data_length)?; //读取图片数据

        trace!("组成密码盒");
        let key_box = {
            let key_length = key_data.len();
            // let key_data = Vec::from(key_data);
            let mut key_box = (0..=255).collect::<Vec<u8>>();
            let mut temp = 0;
            let mut last_byte = 0;
            let mut key_offset = 0;

            for i in 0..=255 {
                let swap = key_box[i as usize] as u64;
                temp = (swap + last_byte + key_data[key_offset] as u64) & 0xFF;
                key_offset += 1;
                if key_offset >= key_length {
                    key_offset = 0;
                }
                key_box[i as usize] = key_box[temp as usize];
                key_box[temp as usize] = swap as u8;
                last_byte = temp;
            }
            // let key_box = key_box.clone();
            key_box
        };

        //解密音乐数据
        trace!("解密音乐数据");
        let _ = messager.send(Signals::Decrypt);
        let mut music_data: Vec<u8> = Vec::new();
        loop {
            let mut chunk = self.seekread_no_error(0x8000);

            let chunk_length = chunk.len();
            if chunk_length != 0 {
                for i in 1..chunk_length + 1 {
                    let j = i & 0xFF;

                    chunk[i - 1] ^= key_box[(key_box[j] as usize
                        + key_box[(key_box[j] as usize + j) & 0xff] as usize)
                        & 0xff]
                    // chunk[i - 1] ^= key_box[(key_box[j] + key_box[(key_box[j as usize] as usize + j as usize) & 0xFF]) & 0xFF];
                }
                //向music_data中最追加chunk
                music_data.append(&mut chunk);
            } else {
                break;
            }
        }

        //退出循环，写入文件

        let _ = messager.send(Signals::Save);
        self.save(&path, music_data)?;

        {
            // 保存封面
            let mut tag = match Tag::new().read_from_path(&path) {
                Ok(o) => o,
                Err(_) => return Err(AppError::CoverCannotSave),
            };
            let cover = Picture {
                mime_type: MimeType::Jpeg,
                data: &image_data,
            };
            tag.set_album_cover(cover); //添加封面
            let _ = tag
                .write_to_path(path.to_str().ok_or(AppError::SaveError)?)
                .map_err(|_| AppError::SaveError); //保存
        }

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

/// 存储元数据的结构体
#[derive(Serialize, Deserialize, Debug)]
#[allow(unused_variables, dead_code)]
struct Metadata {
    //编号
    #[serde(rename = "musicId", skip)] //没用过，跳过
    music_id: String,
    // 音乐名称
    #[serde(rename = "musicName")]
    music_name: String,
    // 艺术家
    #[serde(rename = "artist")]
    music_artist: Vec<(String, String)>,
    // 专辑id
    #[serde(rename = "albumId")]
    album_id: String,
    // 专辑
    #[serde(rename = "album")]
    album: String,
    //
    #[serde(rename = "albumPicDocId", skip)]
    album_pic_doc_id: String,
    //
    #[serde(rename = "albumPic", skip)]
    album_pic: String,
    // 比特率
    #[serde(rename = "bitrate")]
    bitrate: u128,
    //
    #[serde(rename = "mp3DocId", skip)]
    mp3_doc_id: String,
    // 时间长短
    #[serde(rename = "duration")]
    duration: u128,
    //
    #[serde(rename = "mvId")]
    mv_id: String,
    // 别名
    #[serde(rename = "alias")]
    alias: Vec<String>,
    // 译名
    #[serde(rename = "transNames")]
    trans_names: Vec<String>,
    // 音乐格式
    #[serde(rename = "format")]
    format: String,
}

// 存储各种密钥的结构体
#[derive(Clone)]
#[allow(dead_code)]
pub struct Key {
    pub core: Vec<u8>,
    pub meta: Vec<u8>,
}

// fn read_meta(file: &mut File, meta_length: u32) -> Result<Vec<u8>, Error> {}

fn convert_to_generic_arrays(input: &[u8]) -> Result<Vec<GenericArray<u8, U16>>, AppError> {
    // 确保输入的长度是16的倍数
    if input.len() % 16 != 0 {
        return Err(AppError::FileDataError);
    }

    Ok(input
        .chunks(16)
        .map(|chunk| {
            // 将每个块转换为GenericArray
            GenericArray::clone_from_slice(chunk)
        })
        .collect())
}

/// ## AES128解密
/// 解密NCM文件的rc4密钥前记得按字节对0x64进行异或
fn aes128_to_slice<T: AsRef<[u8]>>(key: &T, blocks: Vec<u8>) -> Result<Vec<u8>, AppError> {
    trace!("进行AES128解密");
    let key = GenericArray::from_slice(key.as_ref());

    let mut blocks = convert_to_generic_arrays(&blocks)?;

    // 初始化密钥
    let cipher = Aes128::new(key);

    // 开始解密
    cipher.decrypt_blocks(&mut blocks);

    //取出解密后的值
    let mut x: Vec<u8> = Vec::new();
    for block in blocks.iter() {
        for i in block {
            x.push(i.to_owned());
        }
    }
    Ok(x)
}

// ## 规范文件名称
// 防止创建文件失败
// 符号一一对应：
// -  \  /  *  ?  "  :   <  >  |
// -  _  _  ＊  ？ ＂  ：  ⟨  ⟩   _
// #[allow(dead_code)]
// fn standardize_filename(old_fullfilename: String) -> String {
//     trace!("格式化文件名");
//     let mut new_fullfilename = String::from(old_fullfilename);
//     // debug!("规范文件名：{}", new_fullfilename);
//     let standard = ["\\", "/", "*", "?", "\"", ":", "<", ">", "|"];
//     let resolution = ["_", "_", "＊", "？", "＂", "：", "⟨", "⟩", "_"];
//     for i in 0..standard.len() {
//         new_fullfilename =
//             new_fullfilename.replace(&standard[i].to_string(), &resolution[i].to_string());
//     }
//     new_fullfilename
// }
