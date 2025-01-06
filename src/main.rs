use ::clap::Parser;
#[allow(unused_imports)]
use log::{error, info, warn};
use messager::Signals;
use std::time::Duration;
use std::{
    path::Path,
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
};

use colored::Colorize;

use walkdir::WalkDir; //遍历目录

mod clap;
mod logger;
mod messager;
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
    let forcesave = cli.forcesave;
    if forcesave{
        warn!("文件强制覆盖已开启！")
    }

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
        // 初始化通讯
        let (tx, rx) = mpsc::channel();

        // 循环开始
        for filepath in undumpfile {
            let output = outputdir.clone();
            let successful = Arc::clone(&successful);
            let sender: Sender<messager::Message> = tx.clone();
            pool.execute(move || match Ncmfile::new(filepath.as_str()) {
                Ok(mut n) => match n.dump(Path::new(&output), sender,forcesave) {
                    Ok(_) => {
                        let mut num = successful.lock().unwrap();
                        *num += 1;
                    }
                    Err(e) => error!("[{}] 解密失败: {}", filepath.yellow(), e),
                },
                Err(e) => error!("[{}] 解密失败: {}", filepath.yellow(), e),
            });
        }
        //循环到此结束
        //进度条

        use indicatif::ProgressBar;
        let progressbar = ProgressBar::new((taskcount) as u64)
            .with_elapsed(Duration::from_millis(50))
            .with_message("破解中");
        //接受消息

        for messages in rx {
            match messages.signal {
                Signals::Start => {
                    // progressbar.inc(1);
                    info!("[{}] 开始读取文件", messages.name)
                }
                Signals::Decrypt => {
                    // progressbar.inc(1);
                    info!("[{}] 开始解密", messages.name)
                }
                Signals::Save => {
                    // progressbar.inc(1);
                    info!("[{}] 保存文件", messages.name)
                }
                Signals::End => {
                    progressbar.inc(1);
                    info!("[{}] 成功!", messages.name)
                }
            }
        }

        progressbar.finish_and_clear();
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
