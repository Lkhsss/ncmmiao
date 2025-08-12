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
    pub fn compare(&self) -> Result<u128, AppError> {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| AppError::SystemTimeError)?
            .as_millis();
        Ok(time - self.0)
    }
}
