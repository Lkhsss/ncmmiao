use crate::messager;
use std::fmt::Debug;
use std::sync::mpsc;
pub struct Messager {
    name: String,
    sender: mpsc::Sender<messager::Message>,
}

pub struct Message {
    pub name: String,
    pub signal: Signals,
}

pub enum Signals {
    Start,
    Decrypt,
    Save,
    End,
}

impl Messager {
    pub fn new(name: String, sender: mpsc::Sender<messager::Message>) -> Self {
        Self { name, sender }
    }
    pub fn send(&self, s: Signals) -> Result<(), std::sync::mpsc::SendError<messager::Message>> {
        self.sender.send(Message {
            name: self.name.clone(),
            signal: s,
        })
    }
}
impl Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match &self.signal {
            Signals::Start => "开始破解".to_string(),
            Signals::Decrypt => "开始解密".to_string(),
            Signals::Save => "保存文件".to_string(),
            Signals::End => "破解完成".to_string(),
        };
        write!(f, "[{}] {}", self.name, message)
    }
}
