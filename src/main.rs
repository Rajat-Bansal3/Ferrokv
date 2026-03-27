use tokio;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    let res = config::Config::builder(Some("/home/rajat/Documents/kv_store/ferrokv.config.toml"))?;
    server::run().await
}
