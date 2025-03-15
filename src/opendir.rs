use log::error;
use std::{path::PathBuf, process::Command};

#[cfg(target_os = "windows")]
pub fn opendir(dir: PathBuf) {
    match Command::new("explorer")
        .arg(&dir) // <- Specify the directory you'd like to open.
        .spawn()
    {
        Err(_) => error!("无法打开输出文件夹：[{}]", dir.display()),
        _ => (),
    }
}

#[cfg(target_os = "linux")]
pub fn opendir(dir: PathBuf) {
    match Command::new("open")
        .arg(&dir) // <- Specify the directory you'd like to open.
        .spawn()
    {
        Err(_) => error!("无法打开输出文件夹：[{}]", dir.display()),
        _ => (),
    }
}
#[cfg(target_os = "macos")]
pub fn opendir(dir: PathBuf) {
    match Command::new("open")
        .arg(&dir) // <- Specify the directory you'd like to open.
        .spawn()
    {
        Err(_) => error!("无法打开输出文件夹：[{}]", dir.display()),
        _ => (),
    }
}
