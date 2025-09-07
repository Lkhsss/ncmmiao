use ::clap::Parser;
use crossbeam_channel::{bounded, Sender};
use crossterm::style::{Color, Stylize}; //防止windows终端乱码
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use log::{error, info, warn, LevelFilter};
use messager::{Message, Messager, Signals};
use std::process::exit;
use std::time::Duration;
use std::{path::Path, sync::Arc};

mod apperror;
mod clap;
mod logger;
mod messager;
mod ncmdump;
mod opendir;
mod pathparse;
mod test;
mod threadpool;
mod time;
use apperror::AppError;
use ncmdump::Ncmfile;
use time::TimeCompare;

fn main() -> Result<(), AppError> {
    // 初始化日志系统
    match logger::init_logger() {
        Ok(_) => (),
        Err(_) => {
            println!("初始化日志系统失败")
        }
    };

    let timer = match TimeCompare::new() {
        Ok(t) => t,
        Err(e) => {
            error!("无法初始化时间戳系统。{}", e);
            exit(1)
        }
    };

    let cli = clap::Cli::parse();

    //设置彩色输出
    // let if_colorful = !cli.nocolor;
    // colored::control::set_override(if_colorful);
    // crossterm::terminal::enable_raw_mode()
    //FIXME 更改颜色库
    //TODO控制颜色输出,更改为使用环境变量

    //获取cpu核心数
    let cpus = num_cpus::get();
    // 最大线程数
    let max_workers = match cli.workers {
        Some(n) => {
            if n >= 1 {
                n
            } else {
                1
            }
        }
        None => cpus, //默认使用cpu核心数作为线程数
    };
    //输入目录
    let input = cli.input;
    //输出目录
    let outputdir = cli.output;
    // 强制覆盖
    let forcesave = cli.forcesave;
    if forcesave {
        warn!("文件{}已开启！", "强制覆盖".with(Color::Red))
    }
    let level = match cli.debug {
        0 | 3 => LevelFilter::Info,
        1 => LevelFilter::Error,
        2 => LevelFilter::Warn,
        4 => LevelFilter::Debug,
        5 => LevelFilter::Trace,
        _ => LevelFilter::Off,
    };
    info!("日志等级：{}", level);
    log::set_max_level(level);

    let undumpfile = pathparse::pathparse(input); // 该列表将存入文件的路径

    let taskcount = undumpfile.len();
    let mut success_count = 0; //成功任务数
    let mut ignore_count = 0; //忽略的任务数
    let mut failure_count = 0; //发生错误的

    if taskcount == 0 {
        if cli.autoopen {
            opendir::autoopen(cli.autoopen, outputdir);
        } else {
            error!("没有找到有效文件。使用-i参数输入需要解密的文件或文件夹。使用-a参数自动打开输出文件夹。");
        }

        exit(2);
    };
    // 创建完整的父目录
    if std::fs::create_dir_all(&outputdir).is_err() {
        return Err(AppError::CannotCreateDir);
    }
    // 初始化线程池
    let pool = threadpool::Pool::new(max_workers);

    info!("将启用{}线程", max_workers.to_string().with(Color::Green));
    // 初始化通讯
    // let (tx, rx) = mpsc::channel();
    let (tx, rx) = bounded(taskcount * 6);

    // 循环开始
    for filepath in undumpfile {
        let output = outputdir.clone();
        let senderin: Sender<Message> = tx.clone();
        let senderon: Sender<Message> = tx.clone();
        // 多线程
        pool.execute(move || match Ncmfile::new(filepath.as_str()) {
            Ok(mut n) => match n.dump(Path::new(&output), senderin, forcesave) {
                Ok(_) => {}
                Err(e) => {
                    let messager = Messager::new(n.fullfilename, senderon);
                    let _ = messager.send(Signals::Err(e));
                }
            },
            Err(e) => {
                let messager = Messager::new(filepath, senderon);
                let _ = messager.send(Signals::Err(e));
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
        match messages.signal {
            Signals::End => success_count += 1,
            Signals::Err(AppError::ProtectFile) => ignore_count += 1,
            Signals::Err(_) => failure_count += 1,
            _ => (),
        }
        if (success_count + ignore_count + failure_count) < taskcount {
            progressbar.inc(1);
            // messages.log(); //发送log
        } else {
            break;
        }
    }
    progressbar.finish_and_clear();

    let timecount = timer.compare().unwrap();
    let showtime = || {
        if timecount > 2000 {
            format!("共计用时{}秒", timecount / 1000)
        } else {
            format!("共计用时{}毫秒", timecount)
        }
    };
    info!(
        "成功解密{}个文件,跳过{}个文件,{}个文件解密失败，{}",
        success_count.to_string().with(Color::Green),
        ignore_count.to_string().with(Color::Magenta),
        failure_count.to_string().with(Color::Red),
        showtime()
    );

    // 自动打开输出文件夹
    opendir::autoopen(cli.autoopen, outputdir);
    Ok(())
}

lazy_static! {
    static ref MP: Arc<MultiProgress> = Arc::new(MultiProgress::new());
}
