use anyhow::Result;

mod luau;
mod net;
mod util;

use net::gateway::Gateway;

pub async fn start_gameserver() -> Result<()> {
    let mut gateway = Gateway::new("0.0.0.0", 23301).await?;
    Box::pin(gateway.listen()).await
}
