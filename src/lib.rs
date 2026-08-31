//! # NcmMiao 核心库
//!
//! 本 crate 同时提供可执行程序（CLI，见 `src/main.rs`）与可复用的解密库。
//!
//! 最核心的 API 是 [`Ncmfile`]，可独立于命令行界面使用：
//!
//! ```no_run
//! use ncmmiao::{AppError, Message, Ncmfile};
//! use std::path::Path;
//! use std::sync::atomic::AtomicBool;
//! use std::sync::Arc;
//!
//! # fn example() -> Result<(), AppError> {
//! let mut ncm = Ncmfile::new("song.ncm")?;
//! let (tx, _rx) = crossbeam_channel::unbounded::<Message>();
//! let cancel = Arc::new(AtomicBool::new(false));
//! ncm.dump(Path::new("out"), tx, false, cancel)?;
//! # Ok(())
//! # }
//! ```
//!
//! 其余模块为支撑逻辑：解密算法（`cipher`）、进度信号（`messager`）、
//! 文件收集（`pathparse`）、线程池（`threadpool`）、计时（`time`）与错误类型（`apperror`）。

pub mod apperror;
pub mod cipher;
pub mod messager;
pub mod ncmdump;
pub mod pathparse;
pub mod threadpool;
pub mod time;

pub use apperror::AppError;
pub use messager::Signal;
pub use ncmdump::Ncmfile;
