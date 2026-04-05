use std::sync::Arc;
mod command;
mod connection;
mod dispatcher;
mod error;
mod listener;
use anyhow::Ok;
use config::ServerConfig;
use proto::ProtoError;
use storage::Store;

use crate::listener::Listener;

pub type ConnectionResult<T> = Result<T, ProtoError>;

pub async fn run(config: ServerConfig, store: Arc<dyn Store>) -> anyhow::Result<()> {
    let listner = Listener::new(config, store).await?;
    listner.run().await?;
    Ok(())
}
