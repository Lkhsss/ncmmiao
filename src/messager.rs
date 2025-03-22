use colored::Colorize;
use log::{error, info, warn};

use crate::{messager, ncmdump};
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
impl Message {
    // 定义一个公共方法 log，用于记录不同信号状态下的日志信息
    pub fn log(&self) {
        let loginfo = match &self.signal {
            Signals::Start => "读取文件",
            Signals::GetMetaInfo => "解密歌曲元信息",
            Signals::GetCover => "解密封面图片数据",
            Signals::Decrypt => "解密歌曲信息",
            Signals::Save => "保存文件",
            Signals::End => "成功!",
            Signals::Err(e)=>&e.to_string(),
        };
        match &self.signal{
            Signals::Err(e)=>{match e{
                ncmdump::NcmError::ProtectFile=>warn!("[{}] {}", self.name.cyan(), loginfo),
                _=>error!("[{}] {}", self.name.cyan(), loginfo),
            }},
            _=>info!("[{}] {}", self.name.cyan(), loginfo)
        }
        
    }
}
#[derive(PartialEq)]
pub enum Signals {
    Start,
    GetMetaInfo,
    GetCover,
    Decrypt,
    Save,
    End,
    Err(ncmdump::NcmError),
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
            Signals::Start => "开始破解",
            Signals::Decrypt => "开始解密",
            Signals::Save => "保存文件",
            Signals::End => "破解完成",
            Signals::GetMetaInfo => "获取元数据",
            Signals::GetCover => "获取封面",
            Signals::Err(e)=>&e.to_string(),

        };
        write!(f, "[{}] {}", self.name, message)
    }
}
