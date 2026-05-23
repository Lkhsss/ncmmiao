use std::{
    ops::Deref,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::AppError;
// 比较时间用的结构体
pub struct TimeCompare(Vec<u128>);

impl TimeCompare {
    /// 新建一个时间比较器
    pub fn new() -> Result<Self, AppError> {
        Ok(Self(vec![
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| AppError::SystemTimeError)?
                .as_millis(),
        ]))
    }

    // 快捷方式，获取当前时间
    pub fn get_time() -> Result<u128, AppError> {
        Ok(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| AppError::SystemTimeError)?
            .as_millis())
    }

    /// 将时间推入栈中
    fn push_time(&mut self, time: u128) {
        self.0.push(time);
    }
    /// 从最开始的时间比较
    pub fn compare_start(&mut self) -> Result<u128, AppError> {
        let time = Self::get_time()?;
        self.push_time(time);
        Ok(time - self.0[0])
    }

    /// 读取上一次时间之间的间隔
    /// 会自动记录当前调用的时间
    pub fn compare_last(&mut self) -> Result<u128, AppError> {
        let time = Self::get_time()?;
        self.push_time(time);
        Ok(time - self.0.last().unwrap()) //只要初始化了就说明一定有值可以读取
    }
}
