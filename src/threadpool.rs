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
                let message = receiver.recv().unwrap();
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
        if max_workers == 0 {
            panic!("最大线程数不能小于零！")
        }
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
        self.sender.send(job).unwrap();
    }

    /// 发送关机信号并等待所有线程退出
    pub fn shutdown(&mut self) {
        for _ in &self.workers {
            self.sender.send(Message::Shutdown).unwrap();
        }
        for w in self.workers.iter_mut() {
            if let Some(t) = w.t.take() {
                t.join().unwrap();
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
