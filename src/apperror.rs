#[derive(Debug, PartialEq, thiserror::Error)]
#[allow(dead_code)]
pub enum AppError {
    #[error("该文件不为NCM格式")]
    NotNcmFile,
    #[error("无法读取文件名称")]
    CannotReadFileName,
    #[error("无法读取歌曲元信息")]
    CannotReadMetaInfo,
    #[error("封面无法保存")]
    CoverCannotSave,
    #[error("读取文件时发生错误")]
    FileReadError,
    #[error("跳过数据时出错。可能是文件大小小于预期")]
    FileSkipError,
    #[error("写入文件时错误")]
    FileWriteError,
    #[error("未找到文件")]
    FileNotFound,
    #[error("已关闭文件强制覆盖且文件已存在。使用-f或-forcesave开启强制覆盖。")]
    ProtectFile,
    #[error("处理文件数据时出错")]
    FileDataError,
    #[error("保存文件出错")]
    SaveError,
    #[error("获取时间戳失败")]
    SystemTimeError,
    #[error("无法创建目录")]
    CannotCreateDir,
    #[error("任务已取消")]
    Cancelled,
}


