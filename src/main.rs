use ::clap::Parser;
use crossbeam_channel::bounded;
use crossterm::style::{Color, Stylize};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use log::{LevelFilter, error, info, warn};
use ncmmiao::time::TimeCompare;
use ncmmiao::{AppError, Ncmfile, Signal, pathparse, threadpool};

use std::process;
use std::sync::atomic::Ordering;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use std::{path::Path, sync::Arc};

mod clap;
mod logger;
mod opendir;

fn main() -> Result<(), AppError> {
    // 初始化日志系统
    match logger::init_logger() {
        Ok(_) => (),
        Err(_) => {
            println!("初始化日志系统失败")
        }
    };

    // 全局取消标志
    let cancel = threadpool::cancel_flag();
    {
        let cancel = cancel.clone();
        let mut ctrlc_count = 0u32;
        ctrlc::set_handler(move || {
            ctrlc_count += 1;
            if ctrlc_count == 1 {
                eprintln!("收到中断信号，正在等待当前任务完成... (再次按 Ctrl+C 强制退出)");
                cancel.store(true, Ordering::SeqCst);
            } else {
                eprintln!("强制退出");
                process::exit(1);
            }
        })
        .expect("无法设置 Ctrl+C 信号处理器");
    }

    let timer = match TimeCompare::new() {
        Ok(t) => t,
        Err(e) => {
            error!("无法初始化时间戳系统。{}", e);
            process::exit(1)
        }
    };

    let cli = clap::Cli::parse();

    //设置彩色输出
    if cli.nocolor {
        // 2024 Edition新标准，被标记为不安全函数
        // 在单线程还是安全的
        unsafe {
            std::env::set_var("NO_COLOR", "true");
        }
    };

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

    let undumpfile = pathparse::pathparse(input);

    let taskcount = undumpfile.len();
    let mut success_count = 0;
    let mut ignore_count = 0;
    let mut failure_count = 0;
    let mut cancelled_count = 0;

    if taskcount == 0 {
        if cli.autoopen {
            opendir::autoopen(cli.autoopen, outputdir);
        } else {
            error!(
                "没有找到有效文件。使用-i参数输入需要解密的文件或文件夹。使用-a参数自动打开输出文件夹。"
            );
        }

        process::exit(2);
    };
    // 创建完整的父目录
    if std::fs::create_dir_all(&outputdir).is_err() {
        return Err(AppError::CannotCreateDir);
    }
    // 初始化线程池
    let pool = threadpool::Pool::new(max_workers);

    info!("将启用{}线程", max_workers.to_string().with(Color::Green));

    // 文件名（用于每文件进度条的显示）
    let names: Vec<String> = undumpfile
        .iter()
        .map(|p| {
            Path::new(p)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(p.as_str())
                .to_string()
        })
        .collect();

    // 初始化通讯：全局通道携带 (文件索引, 信号)，由转发线程完成文件归因
    let (tx, rx) = bounded::<(usize, Signal)>(max_workers * 6);

    // 循环开始：每文件独立 channel + 转发线程，使进度信号能归因到具体文件
    let mut forwarders: Vec<JoinHandle<()>> = Vec::with_capacity(taskcount);
    for (i, filepath) in undumpfile.iter().enumerate() {
        let (file_tx, file_rx) = bounded::<Signal>(16);
        let tx = tx.clone();
        forwarders.push(thread::spawn(move || {
            while let Ok(s) = file_rx.recv() {
                if tx.send((i, s)).is_err() {
                    break;
                }
            }
        }));

        let output = outputdir.clone();
        let cancel = cancel.clone();
        let filepath = filepath.clone();
        // 多线程
        pool.execute(move || match Ncmfile::new(filepath.as_str()) {
            Ok(mut n) => match n.dump_to_file(Path::new(&output), Some(file_tx.clone()), forcesave, cancel)
            {
                Ok(_) => {}
                Err(e) => {
                    let _ = file_tx.send(Signal::Err(e));
                }
            },
            Err(e) => {
                let _ = file_tx.send(Signal::Err(e));
            }
        });
    }

    //总进度条（已完成文件数）
    let total_style = ProgressStyle::default_bar()
        .progress_chars("#>-")
        .template("{spinner:.green} 总进度 [{wide_bar:.cyan/blue}] {percent_precise}% ({eta})")
        .unwrap();
    let progressbar = MP.add(
        ProgressBar::new(taskcount as u64)
            .with_elapsed(Duration::from_millis(50))
            .with_style(total_style),
    );

    //每文件进度条样式（Signal::Start 到达时才创建）
    let file_style = ProgressStyle::default_bar()
        .progress_chars("#>-")
        .template("{spinner:.green} {msg:.cyan} [{wide_bar:.cyan/blue}] {percent_precise}%")
        .unwrap();
    let mut file_bars: Vec<Option<ProgressBar>> = vec![None; taskcount];

    // 接受消息
    for (i, signal) in rx {
        match signal {
            Signal::Start => {
                // 文件开始处理时创建该文件的进度条（长度固定为 100 表示百分比）
                let bar = MP.add(
                    ProgressBar::new(100)
                        .with_style(file_style.clone())
                        .with_message(names[i].clone()),
                );
                file_bars[i] = Some(bar);
            }
            Signal::Decrypt(progress) => {
                // 按字节进度更新该文件进度条（progress 为 0.0~1.0）
                if let Some(bar) = &file_bars[i] {
                    bar.set_position((progress * 100.0).round() as u64);
                }
            }
            Signal::End => {
                success_count += 1;
                if let Some(bar) = file_bars[i].take() {
                    bar.finish_and_clear();
                }
                progressbar.inc(1);
            }
            Signal::Err(AppError::ProtectFile) => {
                ignore_count += 1;
                if let Some(bar) = file_bars[i].take() {
                    bar.finish_and_clear();
                }
                progressbar.inc(1);
            }
            Signal::Err(AppError::Cancelled) => {
                cancelled_count += 1;
                if let Some(bar) = file_bars[i].take() {
                    bar.finish_and_clear();
                }
                progressbar.inc(1);
            }
            Signal::Err(e) => {
                error!("{}", e);
                failure_count += 1;
                if let Some(bar) = file_bars[i].take() {
                    bar.finish_and_clear();
                }
                progressbar.inc(1);
            }
            _ => (),
        }
        if (success_count + ignore_count + failure_count + cancelled_count) >= taskcount {
            break;
        }
    }
    progressbar.finish_and_clear();

    // 等待所有转发线程退出
    for f in forwarders {
        let _ = f.join();
    }

    let timecount = timer.compare_start().unwrap();
    let showtime = || {
        if timecount > 2000 {
            format!("共计用时{}秒", timecount / 1000)
        } else {
            format!("共计用时{}毫秒", timecount)
        }
    };
    info!(
        "成功解密{}个文件,跳过{}个文件,{}个文件解密失败,{}个文件被取消，{}",
        success_count.to_string().with(Color::Green),
        ignore_count.to_string().with(Color::Magenta),
        failure_count.to_string().with(Color::Red),
        cancelled_count.to_string().with(Color::Yellow),
        showtime()
    );

    // 自动打开输出文件夹
    opendir::autoopen(cli.autoopen, outputdir);
    Ok(())
}

lazy_static! {
    static ref MP: Arc<MultiProgress> = Arc::new(MultiProgress::new());
}
