use clap::Parser;

#[derive(Parser, Clone, Debug)]
#[command(name = "marketd", about = "Coinswap market offer aggregator daemon")]
pub struct Config {
    #[arg(
        long,
        env = "MARKETD_BITCOIN_RPC_URL",
        default_value = "localhost:18443"
    )]
    pub bitcoin_rpc_url: String,

    #[arg(long, env = "MARKETD_BITCOIN_RPC_USER", default_value = "user")]
    pub bitcoin_rpc_user: String,

    #[arg(long, env = "MARKETD_BITCOIN_RPC_PASS", default_value = "password")]
    pub bitcoin_rpc_pass: String,

    #[arg(
        long,
        env = "MARKETD_ZMQ_ADDR",
        default_value = "tcp://127.0.0.1:28332"
    )]
    pub zmq_addr: String,

    #[arg(long, env = "MARKETD_TOR_CONTROL_PORT", default_value_t = 9051)]
    pub tor_control_port: u16,

    #[arg(long, env = "MARKETD_TOR_AUTH_PASSWORD", default_value = "")]
    pub tor_auth_password: String,

    #[arg(long, env = "MARKETD_SYNC_INTERVAL_SECS", default_value_t = 60)]
    pub sync_interval_secs: u64,

    #[arg(long, env = "MARKETD_LISTEN_ADDR", default_value = "127.0.0.1:3000")]
    pub listen_addr: String,

    #[arg(long, env = "MARKETD_STATIC_DIR", default_value = "web/dist")]
    pub static_dir: String,

    /// Log filter directive (e.g. "debug", "tower_http=debug,info")
    #[arg(
        long,
        default_value = "tower_http=debug,info",
        env = "MARKETD_LOG_FILTER"
    )]
    pub log_filter: String,

    /// Disable ANSI colors in log output (useful for log files / CI)
    #[arg(long, default_value_t = false, env = "MARKETD_NO_COLOR")]
    pub no_color: bool,
}
