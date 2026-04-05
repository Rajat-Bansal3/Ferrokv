use storage::ShardedStore;
use tokio;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    let config = config::Config::load(Some("/home/rajat/Documents/kv_store/ferrokv.config.toml"))?;
    let store = ShardedStore::new(config.storage);
    server::run(config.server, store).await
}
