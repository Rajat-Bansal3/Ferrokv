use config::FsyncPolicy;
use parking_lot::Mutex;
use std::{sync::Arc, time::Duration};
use tokio::task::JoinHandle;

use crate::writer::AofWriter;

pub struct AofFsyncer {
    pub policy: FsyncPolicy,
    pub interval: Duration,
}

impl AofFsyncer {
    pub fn new(policy: FsyncPolicy, interval: Duration) -> Self {
        Self { policy, interval }
    }
    pub fn spawn(&self, writer: Arc<Mutex<AofWriter>>) -> JoinHandle<()> {
        let policy = self.policy.clone();
        let duration = self.interval;
        tokio::spawn(async move {
            let mut last_sync = std::time::Instant::now();
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
                let should_sync = match policy {
                    FsyncPolicy::Always => false,
                    FsyncPolicy::No => false,
                    FsyncPolicy::EverySec => last_sync.elapsed() >= duration,
                };
                if should_sync {
                    let mut writter = writer.lock();
                    if let Err(e) = writter.flush() {
                        eprintln!("AOF flush error: {}", e);
                    }
                    if let Err(e) = writter.fsync() {
                        eprintln!("AOF fsync error: {}", e);
                    }
                    last_sync = std::time::Instant::now();
                }
            }
        })
    }
}
