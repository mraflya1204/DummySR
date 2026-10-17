use anyhow::Result;

mod luau;
mod net;
mod util;

use net::gateway::Gateway;

/// Printed at startup so it's easy to tell which build is running.
const DUMMYSR_BUILD: &str = "DummySR 2026-10-17 (exrank command: extra Eidolons)";

pub async fn start_gameserver() -> Result<()> {
    tracing::info!("{DUMMYSR_BUILD}");
    let mut gateway = Gateway::new("0.0.0.0", 23301).await?;
    Box::pin(gateway.listen()).await
}
