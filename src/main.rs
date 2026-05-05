mod config;
mod server;
mod state;

use crate::{config::Config, state::new_store};
use clap::Parser;
use coinswap::taker::TakerInitConfig;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
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
    let sync_cfg = cfg.clone();

    std::thread::Builder::new()
        .name("marketd-sync".into())
        .spawn(move || sync_loop(sync_cfg, sync_store))?;

    tracing::info!(static_dir = %cfg.static_dir, "Serving frontend from");
    let app = server::router(store, cfg.static_dir.clone());
    let listener = TcpListener::bind(&cfg.listen_addr).await?;
    tracing::info!(addr = %cfg.listen_addr, "HTTP server listening");
    axum::serve(listener, app).await?;

    Ok(())
}

// Poll until a TCP port accepts a connection on the *first* resolved address.
// Matches the behaviour of `bitcoind::bitcoincore_rpc`'s simple_http client, which
// uses only addr.next() — so `localhost` resolving to ::1 first will silently
// pass a multi-addr probe but fail the actual RPC call.
fn wait_for_tcp(addr: &str, label: &str) {
    use std::net::{TcpStream, ToSocketAddrs};
    loop {
        let first = addr.to_socket_addrs().ok().and_then(|mut it| it.next());
        match first.and_then(|a| TcpStream::connect_timeout(&a, Duration::from_secs(5)).ok()) {
            Some(_) => {
                tracing::info!("{label} is reachable ({addr})");
                return;
            }
            None => {
                tracing::info!("Waiting for {label} ({addr})...");
                std::thread::sleep(Duration::from_secs(5));
            }
        }
    }
}

fn sync_loop(cfg: Config, store: state::SharedStore) {
    use bitcoind::bitcoincore_rpc::Auth;
    use coinswap::{
        taker::{Taker, offers::MakerState},
        wallet::RPCConfig,
    };

    let rpc_config = RPCConfig {
        url: cfg.bitcoin_rpc_url.clone(),
        auth: Auth::UserPass(cfg.bitcoin_rpc_user.clone(), cfg.bitcoin_rpc_pass.clone()),
        wallet_name: "marketd-wallet".to_string(),
    };

    wait_for_tcp(&cfg.bitcoin_rpc_url, "Bitcoin RPC");
    wait_for_tcp(
        &format!("127.0.0.1:{}", cfg.tor_control_port),
        "Tor control port",
    );

    let init_config = TakerInitConfig {
        wallet_file_name: Some("marketd-wallet".to_string()),
        rpc_config: Some(rpc_config.clone()),
        control_port: Some(cfg.tor_control_port),
        tor_auth_password: Some(cfg.tor_auth_password.clone()),
        zmq_addr: cfg.zmq_addr.clone(),
        ..TakerInitConfig::default()
    };

    let taker = loop {
        tracing::info!("Initializing Taker...");
        match Taker::init(init_config.clone()) {
            Ok(t) => {
                tracing::info!("Taker initialized successfully");
                break t;
            }
            Err(e) => {
                tracing::warn!(error = ?e, "Taker init failed, retrying in 15s (waiting for Bitcoin node / Tor)");
                std::thread::sleep(Duration::from_secs(15));
            }
        }
    };

    loop {
        if let Err(e) = taker.sync_offerbook_and_wait() {
            tracing::error!(error = ?e, "sync_offerbook_and_wait failed");
            std::thread::sleep(Duration::from_secs(cfg.sync_interval_secs));
            continue;
        }

        let book = match taker.fetch_offers() {
            Ok(b) => b,
            Err(e) => {
                tracing::error!(error = ?e, "fetch_offers failed");
                std::thread::sleep(Duration::from_secs(cfg.sync_interval_secs));
                continue;
            }
        };

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let offers: Vec<state::ApiOffer> = book
            .all_makers()
            .into_iter()
            .filter(|m| matches!(m.state, MakerState::Good))
            .filter_map(|m| {
                m.offer
                    .as_ref()
                    .map(|o| state::ApiOffer::from_coinswap(o, &m.address, timestamp))
            })
            .collect();

        let count = offers.len();
        {
            let mut s = store.write().unwrap();
            s.offers = offers;
            s.last_sync = Some(timestamp);
        }

        tracing::info!(count, "Sync done, sleeping {}s", cfg.sync_interval_secs);
        std::thread::sleep(Duration::from_secs(cfg.sync_interval_secs));
    }
}
