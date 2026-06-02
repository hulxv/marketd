use std::time::{Duration, SystemTime, UNIX_EPOCH};

use coinswap::{
    taker::{TakerInitConfig, api::ConnectionType},
    wallet::RPCConfig,
};

use crate::{config::Config, state};

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

/// Translate the CLI `Config` into a `TakerInitConfig`. Kept separate so that
/// integration tests can build their own `TakerInitConfig` (e.g. Clearnet +
/// an in-process nostr relay) without constructing a fake CLI `Config`.
pub fn build_taker_config(cfg: &Config) -> TakerInitConfig {
    use bitcoind::bitcoincore_rpc::Auth;

    let rpc_config = RPCConfig {
        url: cfg.bitcoin_rpc_url.clone(),
        auth: Auth::UserPass(cfg.bitcoin_rpc_user.clone(), cfg.bitcoin_rpc_pass.clone()),
        wallet_name: "marketd-wallet".to_string(),
    };

    TakerInitConfig {
        wallet_file_name: Some("marketd-wallet".to_string()),
        rpc_config: Some(rpc_config),
        control_port: Some(cfg.tor_control_port),
        tor_auth_password: Some(cfg.tor_auth_password.clone()),
        zmq_addr: cfg.zmq_addr.clone(),
        ..TakerInitConfig::default()
    }
}

pub fn sync_loop(init_config: TakerInitConfig, sync_interval_secs: u64, store: state::SharedStore) {
    use coinswap::taker::{Taker, offers::MakerState};

    if let Some(rpc) = init_config.rpc_config.as_ref() {
        wait_for_tcp(&rpc.url, "Bitcoin RPC");
    }
    if init_config.connection_type == ConnectionType::Tor
        && let Some(port) = init_config.control_port
    {
        wait_for_tcp(&format!("127.0.0.1:{port}"), "Tor control port");
    }

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
            std::thread::sleep(Duration::from_secs(sync_interval_secs));
            continue;
        }

        let book = match taker.fetch_offers() {
            Ok(b) => b,
            Err(e) => {
                tracing::error!(error = ?e, "fetch_offers failed");
                std::thread::sleep(Duration::from_secs(sync_interval_secs));
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

        tracing::info!(count, "Sync done, sleeping {sync_interval_secs}s");
        std::thread::sleep(Duration::from_secs(sync_interval_secs));
    }
}
