#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub enum AppError {
    NotNcmFile,
    CannotReadFileName,
    CannotReadMetaInfo,
    CoverCannotSave,
    FileReadError,
    FileSkipError,
    FileWriteError,
    FullFilenameError,
    FileNotFound,
    ProtectFile,
    FileDataError,
    SaveError,
    SystemTimeError,
    CannotCreateDir,
}

impl std::error::Error for AppError {}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let result = match self {
            Self::NotNcmFile => "该文件不为NCM格式",
            Self::CannotReadFileName => "无法读取文件名称",
            Self::CannotReadMetaInfo => "无法读取歌曲元信息",
            Self::CoverCannotSave => "封面无法保存",
            Self::FileSkipError => "跳过数据时出错。可能是文件大小小于预期",
            Self::FileReadError => "读取文件时发生错误",
            Self::FileWriteError => "写入文件时错误",
            Self::FullFilenameError => "文件名不符合规范",
            Self::FileNotFound => "未找到文件",
            Self::ProtectFile => "已关闭文件强制覆盖且文件已存在。使用-f或-forcesave开启强制覆盖。",
            Self::FileDataError => "处理文件数据时出错",
            Self::SaveError => "保存文件出错",
            Self::SystemTimeError => "获取时间戳失败",
            Self::CannotCreateDir => "无法创建父级目录", // _ =>  "未知错误",
        };
        write!(f, "{}", result)
    }
}
