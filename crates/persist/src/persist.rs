use std::{path::Path, time::Duration};

use bytes::{Bytes, BytesMut};
use command::Command;
use config::{FsyncPolicy, PersistenceConfig};
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};

use crate::{
    serialise::serialize_command,
    writer::{AofResponse, AofWriter},
};

pub struct PersistHandle {
    pub sender: UnboundedSender<Bytes>,
    pub join_handler: JoinHandle<()>,
}

impl PersistHandle {
    pub fn new(config: &PersistenceConfig) -> AofResponse<Self> {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Bytes>();

        let mut writer = AofWriter::open(Path::new(config.aof_path.as_str()))?;
        let policy = config.fsync.clone();
        let fsync_interval = Duration::from_secs(1);

        let join_handler = tokio::spawn(async move {
            let mut last_sync = std::time::Instant::now();
            loop {
                while let Ok(bytes) = rx.try_recv() {
                    if let Err(e) = writer.append(&bytes) {
                        eprintln!("AOF write error: {}", e);
                    }
                }
                match policy {
                    FsyncPolicy::Always => {
                        if let Err(e) = writer.flush().and_then(|_| writer.fsync()) {
                            eprintln!("AOF fsync error: {}", e);
                        }
                    }
                    FsyncPolicy::EverySec => {
                        if last_sync.elapsed() >= fsync_interval {
                            if let Err(e) = writer.flush().and_then(|_| writer.fsync()) {
                                eprintln!("AOF fsync error: {}", e);
                            }
                            last_sync = std::time::Instant::now();
                        }
                    }
                    FsyncPolicy::No => {}
                }
                // yield to avoid busy loop
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        });

        Ok(Self {
            sender: tx,
            join_handler,
        })
    }

    pub fn log_command(&self, cmd: &Command) {
        let mut buf = BytesMut::new();
        serialize_command(cmd, &mut buf);
        self.sender.send(buf.freeze()).ok();
    }

    pub fn shutdown(&self) {
        self.join_handler.abort();
    }
}
