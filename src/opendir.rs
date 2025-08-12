use log::error;
use std::{path::PathBuf, process::Command};

#[cfg(target_os = "windows")]
pub fn opendir(dir: PathBuf) {
    if let Err(_) = Command::new("explorer")
        .arg(&dir) // <- Specify the directory you'd like to open.
        .spawn()
    {
        error!("无法打开输出文件夹：[{}]", dir.display())
    }
}

#[cfg(target_os = "linux")]
pub fn opendir(dir: PathBuf) {
    if let Err(_) = Command::new("open")
        .arg(&dir) // <- Specify the directory you'd like to open.
        .spawn()
    {
        error!("无法打开输出文件夹：[{}]", dir.display())
    }
}
#[cfg(target_os = "macos")]
pub fn opendir(dir: PathBuf) {
    if let Err(_) = Command::new("open")
        .arg(&dir) // <- Specify the directory you'd like to open.
        .spawn()
    {
        error!("无法打开输出文件夹：[{}]", dir.display())
    }
}
