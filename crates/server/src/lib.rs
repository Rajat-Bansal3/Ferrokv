use std::{thread::sleep, time::Duration};

use anyhow::Ok;
use tracing::{debug, info, instrument};

#[instrument]
pub async fn run() -> anyhow::Result<()> {
    info!("starting server");
    sleep(Duration::new(5, 0));
    debug!("ran");
    Ok(())
}
