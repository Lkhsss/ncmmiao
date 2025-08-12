use log::{debug, error, warn};
use std::path::Path;
use walkdir::WalkDir;

use crate::apperror::AppError;

pub fn pathparse(input: Vec<String>) -> Vec<String> {
    let mut undumpfile = Vec::new(); // 该列表将存入文件的路径
                                     // 遍历输入的每一个路径参数
    for arg in input {
        //解析传入的每一个路径：文件or文件夹
        let path = Path::new(&arg);

        if path.is_file() {
            // 当后缀符合为ncm时才加入列表
            if let Some(extension) = path.extension() {
                if extension == "ncm" {
                    let _ = &mut undumpfile.push(arg.to_owned());
                }
            }
        } else if path.is_dir() {
            for entry in WalkDir::new(path) {
                let new_entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        error!("无法遍历目录: {}", e);
                        continue;
                    }
                };
                let filepath = new_entry.into_path();
                // 当后缀符合为ncm时才加入列表
                match filepath.extension() {
                    Some(extension) => {
                        if extension == "ncm" {
                            match filepath.to_str() {
                                Some(s) => {
                                    let _ = &mut undumpfile.push(s.into());
                                }
                                None => {
                                    debug!("无法获取你文件路径");
                                    continue;
                                }
                            };
                        }
                    }
                    None => {
                        continue;
                    }
                }
            }
        }
    }
    undumpfile
}
