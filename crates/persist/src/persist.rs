use std::{path::Path, sync::Arc, time::Duration};

use command::Command;
use config::{FsyncPolicy, PersistenceConfig};
use parking_lot::Mutex;
use tokio::task::JoinHandle;

use crate::{
    syncer::AofFsyncer,
    writer::{AofResponse, AofWriter},
};

pub struct PersistHandle {
    pub writer: Arc<Mutex<AofWriter>>,
    pub policy: FsyncPolicy,
    pub join_handler: JoinHandle<()>,
}

impl PersistHandle {
    pub fn new(config: &PersistenceConfig) -> AofResponse<Self> {
        let writer = Arc::new(Mutex::new(AofWriter::open(Path::new(
            config.aof_path.as_str(),
        ))?));

        let policy = config.fsync.clone();
        let join_handler =
            AofFsyncer::new(policy.clone(), Duration::from_secs(1)).spawn(writer.clone());

        Ok(Self {
            join_handler,
            writer,
            policy,
        })
    }

    pub fn log_command(&self, cmd: &Command) -> AofResponse<()> {
        let mut writer = self.writer.lock();

        writer.append(cmd)?;

        if let FsyncPolicy::Always = self.policy {
            writer.flush()?;
            writer.fsync()?;
        }

        Ok(())
    }

    pub fn shutdown(&self) -> AofResponse<()> {
        self.join_handler.abort();

        let mut writer = self.writer.lock();

        writer.flush().map_err(|e| {
            eprintln!("AOF flush error on shutdown: {}", e);
            e
        })?;

        writer.fsync().map_err(|e| {
            eprintln!("AOF fsync error on shutdown: {}", e);
            e
        })?;

        Ok(())
    }
}
