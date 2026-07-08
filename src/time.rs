use std::time::{SystemTime, UNIX_EPOCH};

use crate::AppError;

pub struct TimeCompare(u128);

impl TimeCompare {
    pub fn new() -> Result<Self, AppError> {
        Ok(Self(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| AppError::SystemTimeError)?
                .as_millis(),
        ))
    }

    /// 返回从构造到现在的毫秒差
    pub fn compare_start(&self) -> Result<u128, AppError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| AppError::SystemTimeError)?
            .as_millis();
        Ok(now - self.0)
    }
}
