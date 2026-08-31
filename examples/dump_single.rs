//! 以库的方式使用 ncmmiao：解密单个 ncm 文件。
//!
//! 运行方式：
//! ```bash
//! cargo run --example dump_single -- <输入.ncm> <输出目录>
//! ```

use ncmmiao::{AppError, Ncmfile, Signal};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

fn main() -> Result<(), AppError> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("用法: dump_single <输入.ncm> <输出目录>");
        std::process::exit(2);
    }
    let input = &args[1];
    let output_dir = &args[2];

    // 创建输出目录（库 API 不负责创建目录）
    std::fs::create_dir_all(output_dir).map_err(|_| AppError::CannotCreateDir)?;

    // 1. 新建 Ncmfile（库 API）
    let mut ncm = Ncmfile::new(input)?;

    // 2. 准备进度信号 channel 与取消标志
    let (tx, _rx) = crossbeam_channel::unbounded::<Signal>();
    let cancel = Arc::new(AtomicBool::new(false));

    // 3. 调用核心解密逻辑（库 API），等价于命令行 `ncmmiao -i input -o output_dir`
    ncm.dump(Path::new(output_dir), tx, false, cancel)?;

    println!("解密完成，输出目录: {}", output_dir);
    Ok(())
}
