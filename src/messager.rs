use crate::AppError;
use std::fmt::Debug;

pub struct Messager {
    sender: crossbeam_channel::Sender<Message>,
}

pub struct Message {
    pub signal: Signals,
}

#[derive(PartialEq)]
pub enum Signals {
    Start,
    GetMetaInfo,
    GetCover,
    Decrypt,
    Save,
    End,
    Err(AppError),
}

impl Messager {
    pub fn new(sender: crossbeam_channel::Sender<Message>) -> Self {
        Self { sender }
    }
    pub fn send(&self, s: Signals) -> Result<(), crossbeam_channel::SendError<Message>> {
        self.sender.send(Message { signal: s })
    }
}
impl Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match &self.signal {
            Signals::Start => "开始破解",
            Signals::Decrypt => "开始解密",
            Signals::Save => "保存文件",
            Signals::End => "破解完成",
            Signals::GetMetaInfo => "获取元数据",
            Signals::GetCover => "获取封面",
            Signals::Err(e) => &e.to_string(),
        };
        write!(f, "{}", message)
    }
}
