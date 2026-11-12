use anyhow::Result;

mod luau;
mod net;
mod util;

use net::gateway::Gateway;

/// Printed at startup so it's easy to tell which build is running.
const DUMMYSR_BUILD: &str = "DummySR 2026-11-13 (game version 4.7.51, res.json fix)";

pub async fn start_gameserver() -> Result<()> {
    tracing::info!("{DUMMYSR_BUILD}");
    // Load res.json now, so a broken file stops the server at startup with the reason, instead
    // of leaving the game on the loading screen at login.
    std::sync::LazyLock::force(&common::resources::GAME_RES);
    tracing::info!("loaded res.json");
    let mut gateway = Gateway::new("0.0.0.0", 23301).await?;
    Box::pin(gateway.listen()).await
}
