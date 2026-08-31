//! # NcmMiao 核心库
//!
//! 本 crate 同时提供可执行程序（CLI，见 `src/main.rs`）与可复用的解密库。
//!
//! 最核心的 API 是 [`Ncmfile`]，可独立于命令行界面使用：
//! [`Ncmfile::dump`] 解密后直接返回音乐字节；[`Ncmfile::dump_to_file`] 解密并保存到输出目录。
//!
//! ```no_run
//! use ncmmiao::{AppError, Ncmfile, Signal};
//! use std::path::Path;
//! use std::sync::atomic::AtomicBool;
//! use std::sync::Arc;
//!
//! # fn example() -> Result<(), AppError> {
//! let mut ncm = Ncmfile::new("song.ncm")?;
//! let (tx, _rx) = crossbeam_channel::unbounded::<Signal>();
//! let cancel = Arc::new(AtomicBool::new(false));
//! ncm.dump_to_file(Path::new("out"), Some(tx), false, cancel)?;
//! # Ok(())
//! # }
//! ```
//!
//! 其余模块为支撑逻辑：解密算法（`cipher`）、进度信号（`messager`）、
//! 文件收集（`pathparse`）、线程池（`threadpool`）、计时（`time`）与错误类型（`apperror`）。

pub mod apperror;
pub mod cipher;
pub mod io;
pub mod model;
pub mod ncmdump;
pub mod pathparse;
pub mod threadpool;
pub mod time;

pub use apperror::AppError;
pub use model::{Ncmfile, Signal};
