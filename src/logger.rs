use crossterm::style::{Color, Stylize}; //防止windows终端乱码
use indicatif::MultiProgress;
use log::{Log, Metadata, Record, SetLoggerError};
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
            #[cfg(target_os = "windows")]
            let level = match record.level() {
                log::Level::Error => ("Error").with(Color::Red),
                log::Level::Warn => ("Warn").with(Color::Yellow),
                log::Level::Info => ("Info").with(Color::Green),
                log::Level::Debug => ("Debug").with(Color::Magenta),
                log::Level::Trace => ("Trace").with(Color::Cyan),
            };
            #[cfg(not(target_os = "windows"))]
            let level = match record.level() {
                log::Level::Error => ("Error").with(Color::Red),
                log::Level::Warn => ("Warn").with(Color::Yellow),
                log::Level::Info => ("Info").with(Color::Green),
                log::Level::Debug => ("Debug").with(Color::Magenta),
                log::Level::Trace => ("Trace").with(Color::Cyan),
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

    Ok(())
}
