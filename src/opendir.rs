use crossterm::style::{Color, Stylize};
use log::{error, info};
use std::{path::PathBuf, process::Command};

#[cfg(target_os = "windows")]
fn opendir(dir: PathBuf) {
    if Command::new("explorer").arg(&dir).spawn().is_err() {
        error!("无法打开输出文件夹：[{}]", dir.display())
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn opendir(dir: PathBuf) {
    if Command::new("open").arg(&dir).spawn().is_err() {
        error!("无法打开输出文件夹：[{}]", dir.display())
    }
}

/// 自动打开输出文件夹
pub fn autoopen(if_auto_open: bool, path: String) {
    let styled_path = (&path[..]).with(Color::Cyan);
    if if_auto_open {
        info!("自动打开文件夹：[{}]", styled_path);
        opendir(path.into());
    } else {
        info!("输出文件夹：[{}]", styled_path);
    };
}
