use std::ffi::OsStr;
use std::os::windows::fs::FileExt;
use std::str::from_utf8;
use std::{
    fs::File,
    io::{BufReader, Read as _, Seek as _, SeekFrom},
    path::Path,
};

use crate::{AppError, Ncmfile};

impl Ncmfile {
    pub fn new(filepath: &str) -> Result<Ncmfile, AppError> {
        let file = File::open(filepath).map_err(AppError::FileReadError)?;
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
        // 校验文件是否为ncm
        let mut magic_header = vec![0; 8];

        let _ = file
            .seek_read(&mut magic_header, 0)
            .map_err(AppError::FileReadError)?;
        Self::is_ncm(&magic_header)?;

        Ok(Ncmfile {
            reader: BufReader::with_capacity(64 * 1024, file),
            filename,
            size,
        })
    }
    // 允许短读，增加容错
    pub(crate) fn seekread(&mut self, length: u64) -> Result<Vec<u8>, AppError> {
        let pos = self
            .reader
            .stream_position()
            .map_err(AppError::FileReadError)?;
        let rest_len = self.size - pos;
        if rest_len > length {
            let mut buf = vec![0; length as usize];
            self.reader
                .read_exact(&mut buf)
                .map_err(AppError::FileReadError)?;
            Ok(buf)
        } else if rest_len < length && rest_len > 0 {
            let mut buf = vec![0; rest_len as usize];
            self.reader
                .read_exact(&mut buf)
                .map_err(AppError::FileReadError)?;
            Ok(buf)
        } else {
            Ok(Vec::with_capacity(0))
        }
    }
    //跟随linux的标准行为，不约束seek范围
    pub(crate) fn skip(&mut self, length: u64) -> Result<u64, AppError> {
        // let pos = self
        //     .reader
        //     .stream_position()
        //     .map_err(AppError::FileReadError)?;

        self.reader
            .seek(SeekFrom::Current(length as i64))
            .map_err(AppError::FileReadError)?;

        Ok(length)
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
