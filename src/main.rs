use std::{path::Path, sync::{Arc, Mutex}};
use ::clap::Parser;
#[allow(unused_imports)]
use log::{error, info, warn};

use colored::Colorize;

use walkdir::WalkDir; //遍历目录

mod clap;
mod logger;
mod ncmdump;
mod threadpool;
use ncmdump::Ncmfile;
mod test;

fn main() {
    let timer = ncmdump::TimeCompare::new();
    // 初始化日志系统
    logger::Logger::new();

    let cli = clap::Cli::parse();

    // 最大线程数
    let max_workers = match cli.workers {
        Some(n) => {
            if n >= 1 {
                n
            } else {
                1
            }
        }
        None => 4,
    };

    let input = cli.input;

    let outputdir = cli.output.unwrap();

    let mut undumpfile = Vec::new(); // 该列表将存入文件的路径

    for arg in input {
        //解析传入的每一个路径：文件or文件夹
        let path = Path::new(&arg);

        if path.is_file() {
            // 当后缀符合为ncm时才加入列表
            match path.extension() {
                Some(extension) => {
                    if extension == "ncm" {
                        let _ = &mut undumpfile.push(arg.to_owned());
                    }
                }
                None => {}
            }
        } else if path.is_dir() {
            for entry in WalkDir::new(path) {
                let new_entry = entry.unwrap().clone();
                let filepath = new_entry.into_path();
                // 当后缀符合为ncm时才加入列表
                match filepath.extension() {
                    Some(extension) => {
                        if extension == "ncm" {
                            let _ = &mut undumpfile.push(String::from(filepath.to_str().unwrap()));
                        }
                    }
                    None => {
                        continue;
                    }
                }
            }
        }
    }
    let taskcount = undumpfile.len();
    let successful = Arc::new(Mutex::new(0));
    if taskcount == 0 {
        error!("没有找到有效文件。使用-i参数输入需要解密的文件或文件夹。")
    } else {
        // 初始化线程池
        let pool = threadpool::Pool::new(max_workers);
        info!("启用{}线程", max_workers);

        for filepath in undumpfile {
            let output = outputdir.clone();
            let successful = Arc::clone(&successful);
            pool.execute(move || {
                match Ncmfile::new(filepath.as_str()) {
                    Ok(mut n) => match n.dump(Path::new(&output)) {
                        Ok(_) => {
                            let mut num = successful.lock().unwrap();
                            *num += 1;},
                        Err(e) => error!("[{}]解密失败: {}", filepath.yellow(), e),
                    },
                    Err(e) => error!("[{}]解密失败: {}", filepath.yellow(), e),
                }
            });
        }
    }
    let timecount = timer.compare();
    let showtime = || {
        if timecount > 2000 {
            format!("共计用时{}秒", timecount / 1000)
        } else {
            format!("共计用时{}毫秒", timecount)
        }
    };
    let successful = *successful.lock().unwrap();
    info!(
        "成功解密{}个文件,{}个文件解密失败，{}",
        successful.to_string().bright_green(),
        (taskcount - successful).to_string().bright_red(),
        showtime()
    )
}
