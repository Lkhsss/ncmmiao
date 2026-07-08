use crossbeam_channel::{Receiver, Sender, unbounded};
use std::sync::{Arc, atomic::AtomicBool};
use std::thread::{self, JoinHandle};

type Job = Box<dyn FnOnce() + 'static + Send>;
enum Message {
    Shutdown,
    NewJob(Job),
}

struct Worker {
    t: Option<JoinHandle<()>>,
}

impl Worker {
    fn new(receiver: Receiver<Message>) -> Worker {
        let t = thread::spawn(move || {
            loop {
                // channel 断开时优雅退出
                let message = match receiver.recv() {
                    Ok(m) => m,
                    Err(_) => break,
                };
                match message {
                    Message::NewJob(job) => {
                        job();
                    }
                    Message::Shutdown => {
                        break;
                    }
                }
            }
        });

        Worker { t: Some(t) }
    }
}

pub struct Pool {
    workers: Vec<Worker>,
    sender: Sender<Message>,
}

impl Pool {
    pub fn new(max_workers: usize) -> Pool {
        assert!(max_workers > 0, "最大线程数必须大于零");
        let (tx, rx) = unbounded();

        let mut workers = Vec::with_capacity(max_workers);
        for _ in 0..max_workers {
            workers.push(Worker::new(rx.clone()));
        }

        Pool {
            workers,
            sender: tx,
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + 'static + Send,
    {
        let job = Message::NewJob(Box::new(f));
        self.sender.send(job).expect("无法向工作线程发送任务");
    }

    /// 发送关机信号并等待所有线程退出
    pub fn shutdown(&mut self) {
        for _ in &self.workers {
            // worker 可能已退出，忽略发送错误
            let _ = self.sender.send(Message::Shutdown);
        }
        for w in self.workers.iter_mut() {
            if let Some(t) = w.t.take() {
                // 线程 panic 时打印错误而非 double panic
                if let Err(e) = t.join() {
                    eprintln!("工作线程 panic: {e:?}");
                }
            }
        }
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// 全局取消标志，用于优雅关机
pub fn cancel_flag() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}
