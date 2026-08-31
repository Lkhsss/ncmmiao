use crate::AppError;
use std::fmt::Debug;

#[derive(PartialEq)]
pub enum Signal {
    Start,
    GetMetaInfo,
    GetCover,
    Decrypt,
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
            Signal::Decrypt => "开始解密",
            Signal::Save => "保存文件",
            Signal::End => "破解完成",
            Signal::Err(e) => &e.to_string(),
        };
        write!(f, "{}", signal)
    }
}
