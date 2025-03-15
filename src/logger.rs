use colored::Color::{Blue, Cyan, Green, Red, Yellow};
use colored::Colorize;
use indicatif::MultiProgress;
use log::{LevelFilter, Log, Metadata, Record, SetLoggerError};
use std::sync::Arc;

// 自定义Logger，将日志发送到MultiProgress
pub struct MultiProgressLogger {
    mp: Arc<MultiProgress>,
}

impl Log for MultiProgressLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let level = match record.level() {
                log::Level::Error => ("Error").color(Red),
                log::Level::Warn => ("Warn").color(Yellow),
                log::Level::Info => ("Info").color(Green),
                log::Level::Debug => ("Debug").color(Blue),
                log::Level::Trace => ("Debug").color(Cyan),
            };
            let message = format!(
                "[{}][{}] {}",
                chrono::Local::now().format("%H:%M:%S"),
                level,
                record.args()
            );
            self.mp.println(message).expect("Failed to print log");
        }
    }

    fn flush(&self) {}
}

// 初始化日志系统
pub fn init_logger() -> Result<(), SetLoggerError> {
    let logger = MultiProgressLogger {
        mp: crate::MP.clone(),
    };
    log::set_boxed_logger(Box::new(logger))?;
    if cfg!(debug_assertions) {
        log::set_max_level(LevelFilter::Debug);
    } else {
        log::set_max_level(LevelFilter::Info);
    }
    Ok(())
}
