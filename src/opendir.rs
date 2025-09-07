use crossterm::style::{Color, Stylize};
use log::{error, info};
use std::{path::PathBuf, process::Command}; //防止windows终端乱码

#[cfg(target_os = "windows")]
pub fn opendir(dir: PathBuf) {
    if Command::new("explorer")
        .arg(&dir) // <- Specify the directory you'd like to open.
        .spawn()
        .is_err()
    {
        error!("无法打开输出文件夹：[{}]", dir.display())
    }
}

#[cfg(target_os = "linux")]
pub fn opendir(dir: PathBuf) {
    if Command::new("open")
        .arg(&dir) // <- Specify the directory you'd like to open.
        .spawn()
        .is_err()
    {
        error!("无法打开输出文件夹：[{}]", dir.display())
    }
}
#[cfg(target_os = "macos")]
pub fn opendir(dir: PathBuf) {
    if Command::new("open")
        .arg(&dir) // <- Specify the directory you'd like to open.
        .spawn()
        .is_err()
    {
        error!("无法打开输出文件夹：[{}]", dir.display())
    }
}

// 自动打开输出文件夹的跨平台函数
pub fn autoopen(if_auto_open: bool, path: String) {
    let styled_path = (&path[..]).with(Color::Cyan);
    if if_auto_open {
        info!("自动打开文件夹：[{}]", styled_path);
        opendir(path.into());
    } else {
        info!("输出文件夹：[{}]", styled_path);
    };
}
