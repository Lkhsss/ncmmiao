use ::clap::Parser;
use colored::{Color, Colorize};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use log::{error, info, warn};
use messager::Message;
use std::time::Duration;
use std::{
    path::Path,
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
};

mod clap;
mod logger;
mod messager;
mod ncmdump;
mod opendir;
mod pathparse;
mod test;
mod threadpool;
use ncmdump::Ncmfile;

const DEFAULT_MAXWORKER: usize = 8;

fn main() {
    let timer = ncmdump::TimeCompare::new();
    // 初始化日志系统
    logger::init_logger().unwrap();

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
        None => DEFAULT_MAXWORKER,
    };

    let input = cli.input;

    let outputdir = cli.output.unwrap();
    let forcesave = cli.forcesave;
    if forcesave {
        warn!("文件{}已开启！", "强制覆盖".bright_red())
    }

    let undumpfile = pathparse::pathparse(input); // 该列表将存入文件的路径

    let taskcount = undumpfile.len();
    let mut success_count = 0; //成功任务数


    if taskcount == 0 {
        error!("没有找到有效文件。使用-i参数输入需要解密的文件或文件夹。")
    } else {
        // 初始化线程池
        let pool = threadpool::Pool::new(max_workers);
        info!(
            "将启用{}线程",
            max_workers.to_string().color(Color::BrightGreen)
        );
        // 初始化通讯
        let (tx, rx) = mpsc::channel();

        // 循环开始
        for filepath in undumpfile {
            let output = outputdir.clone();
            let senderin: Sender<messager::Message> = tx.clone();
            let senderon: Sender<messager::Message> = tx.clone();
            pool.execute(move || match Ncmfile::new(filepath.as_str()) {
                Ok(mut n) => match n.dump(Path::new(&output), senderin, forcesave) {
                    Ok(_) => {}
                    Err(e) => {
                        let messager = messager::Messager::new(n.fullfilename, senderon);
                        messager.send(messager::Signals::Err(e));
                    }
                },
                Err(e) => {
                    let messager = messager::Messager::new(filepath, senderon);
                    messager.send(messager::Signals::Err(e));
                }
            });
        }
        //循环到此结束
        //进度条

        let pb = ProgressBar::new((taskcount * 6) as u64) //长度乘积取决于Signal的数量
            .with_elapsed(Duration::from_millis(50))
            .with_style(
                ProgressStyle::default_bar()
                    .progress_chars("#>-")
                    .template("{spinner:.green} [{wide_bar:.cyan/blue}] {percent_precise}% ({eta})")
                    .unwrap(),
            )
            .with_message("解密中");
        let progressbar = MP.add(pb);

        //定义计数器
        // 接受消息!!!!!!!!!!
        for messages in rx {
            match messages.signal{
                messager::Signals::End|messager::Signals::Err(_)=>{success_count+=1},
                _=>()
            }
            if success_count < taskcount {
                progressbar.inc(1);
                messages.log(); //发送log
            } else {
                break;
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
    info!(
        "成功解密{}个文件,{}个文件解密失败，{}",
        success_count.to_string().bright_green(),
        (taskcount - success_count).to_string().bright_red(),
        showtime()
    );

    // 自动打开输出文件夹
    if cli.autoopen {
        info!("自动打开文件夹：[{}]", outputdir.cyan());
        opendir::opendir(outputdir.into());
    } else {
        info!("输出文件夹：[{}]", outputdir.cyan());
    };
}

lazy_static! {
    static ref MP: Arc<MultiProgress> = Arc::new(MultiProgress::new());
}
