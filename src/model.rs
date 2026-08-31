use crate::AppError;
use std::fmt::Debug;
use std::fs::File;
use std::io::BufReader;

#[derive(Debug)]
pub struct Ncmfile {
    pub(crate) reader: BufReader<File>,
    pub filename: String,
    pub size: u64,
}


pub enum Signal {
    Start,
    GetMetaInfo,
    GetCover,
    Decrypt(f64),
    Save,
    End,
    Err(AppError),
}

impl Debug for Signal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let signal = match &self {
            Signal::Start => "开始破解",
            Signal::GetMetaInfo => "获取元数据",
            Signal::GetCover => "获取封面",
            Signal::Decrypt(_) => "开始解密",
            Signal::Save => "保存文件",
            Signal::End => "破解完成",
            Signal::Err(e) => &e.to_string(),
        };
        write!(f, "{}", signal)
    }
}
