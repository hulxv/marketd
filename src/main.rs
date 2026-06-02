use clap::Parser;
use marketd::{
    config::Config,
    server,
    state::new_store,
    sync::{build_taker_config, sync_loop},
};
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Config::parse();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(!cfg.no_color)
                .with_thread_names(true)
                .with_target(false),
        )
        .with(tracing_subscriber::EnvFilter::new(&cfg.log_filter))
        .init();

    tracing::info!(listen = %cfg.listen_addr, "Starting marketd");

    let store = new_store();
    let sync_store = store.clone();
    let init_config = build_taker_config(&cfg);
    let sync_interval = cfg.sync_interval_secs;

    std::thread::Builder::new()
        .name("marketd-sync".into())
        .spawn(move || sync_loop(init_config, sync_interval, sync_store))?;

    tracing::info!(static_dir = %cfg.static_dir, "Serving frontend from");
    let app = server::router(store, cfg.static_dir.clone());
    let listener = TcpListener::bind(&cfg.listen_addr).await?;
    tracing::info!(addr = %cfg.listen_addr, "HTTP server listening");
    axum::serve(listener, app).await?;

    Ok(())
}
