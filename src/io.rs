use std::ffi::OsStr;
use std::str::from_utf8;
use std::{
    fs::File,
    io::{BufReader, Read as _, Seek as _, SeekFrom},
    path::Path,
};

use crate::{AppError, Ncmfile};

impl Ncmfile {
    pub fn new(filepath: &str) -> Result<Ncmfile, AppError> {
        let mut file = File::open(filepath).map_err(AppError::FileReadError)?;
        let size = file
            .metadata()
            .map_err(|_| AppError::CannotReadMetaInfo)?
            .len();
        let filename = Path::new(filepath)
            .file_stem()
            .unwrap_or(OsStr::new("unknown"))
            .to_str()
            .ok_or(AppError::CannotReadFileName)?
            .to_string();
        // 校验文件是否为ncm：读取前8字节后复位游标（跨平台，不改变读取位置）
        let mut magic_header = [0u8; 8];
        file.read_exact(&mut magic_header)
            .map_err(AppError::FileReadError)?;
        file.seek(SeekFrom::Start(0))
            .map_err(AppError::FileReadError)?;
        Self::is_ncm(&magic_header)?;

        Ok(Ncmfile {
            reader: BufReader::with_capacity(64 * 1024, file),
            filename,
            size,
        })
    }
    pub(crate) fn seek(&mut self, pos: SeekFrom) -> Result<u64, AppError> {
        self.reader.seek(pos).map_err(AppError::FileReadError)
    }
    // 允许短读，增加容错：请求长度超过剩余字节时读取实际剩余部分
    pub(crate) fn seekread(&mut self, length: u64) -> Result<Vec<u8>, AppError> {
        let pos = self
            .reader
            .stream_position()
            .map_err(AppError::FileReadError)?;
        // saturating_sub：游标可能被 skip 越过文件末尾，避免 u64 下溢
        let read_len = self.size.saturating_sub(pos).min(length);
        if read_len == 0 {
            return Ok(Vec::new());
        }
        let mut buf = vec![0; read_len as usize];
        self.reader
            .read_exact(&mut buf)
            .map_err(AppError::FileReadError)?;
        Ok(buf)
    }
    //跟随linux的标准行为，不约束seek范围
    pub(crate) fn skip(&mut self, length: u64) -> Result<u64, AppError> {
        let length = i64::try_from(length).map_err(|_| AppError::NumParseError)?;
        self.reader
            .seek(SeekFrom::Current(length))
            .map_err(AppError::FileReadError)?;
        Ok(length as u64)
    }

    pub fn is_ncm(data: &[u8]) -> Result<(), AppError> {
        let header = from_utf8(data).map_err(|_| AppError::NotNcmFile)?;
        if header != "CTENFDAM" {
            Err(AppError::NotNcmFile)
        } else {
            Ok(())
        }
    }

    pub fn get_filename(&self) -> &str {
        &self.filename
    }
}
