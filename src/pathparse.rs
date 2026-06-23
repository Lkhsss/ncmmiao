use log::{debug, error};
use std::path::Path;
use walkdir::WalkDir;

pub fn pathparse(input: Vec<String>) -> Vec<String> {
    let mut undumpfile = Vec::with_capacity(input.len());

    for arg in input {
        let path = Path::new(&arg);

        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == "ncm" {
                    undumpfile.push(arg);
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
                match filepath.extension() {
                    Some(extension) => {
                        if extension == "ncm" {
                            match filepath.to_str() {
                                Some(s) => undumpfile.push(s.into()),
                                None => {
                                    debug!("无法获取文件路径");
                                    continue;
                                }
                            };
                        }
                    }
                    None => continue,
                }
            }
        }
    }
    undumpfile
}
