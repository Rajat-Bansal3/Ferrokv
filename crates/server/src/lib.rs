use std::{path::Path, sync::Arc};
mod connection;
mod dispatcher;
mod error;
mod listener;
use config::Config;
use proto::ProtoError;
use storage::Store;

use crate::listener::Listener;

pub type ConnectionResult<T> = Result<T, ProtoError>;

pub async fn run(config: Config, store: Arc<dyn Store>) -> anyhow::Result<()> {
    match persist::reader::replay(Path::new(&config.persistence.aof_path), &store) {
        Ok(count) => println!("AOF replay: {} commands", count),
        Err(e) => println!("AOF replay error: {}", e),
    }
    let listner = Listener::new(config, store).await?;
    listner.run().await?;
    Ok(())
}
