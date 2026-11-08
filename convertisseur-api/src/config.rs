use std::{net::SocketAddr, path::PathBuf};

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(version, about = "Convertisseur HTTP API and web server")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// `tracing` filter directive (e.g. `info`, `convertisseur_api=debug,sqlx=warn`)
    ///
    /// Syntax: <https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives>
    #[arg(
        long,
        env = "CONVERTISSEUR_LOG_FILTER",
        default_value = "info",
        global = true
    )]
    pub log_filter: String,

    /// Log output format
    #[arg(
        long,
        env = "CONVERTISSEUR_LOG_FORMAT",
        value_enum,
        default_value_t = LogFormat::Text,
        global = true
    )]
    pub log_format: LogFormat,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum LogFormat {
    Text,
    Json,
}

#[derive(Subcommand)]
pub enum Command {
    /// Apply the database migrations, set up the super admin and serve the
    /// API and the webapp
    Serve(ServeArgs),
    /// Probe the liveness endpoint of a server running on this host; exits
    /// non-zero when it does not answer 200
    Healthcheck(HealthcheckArgs),
}

#[derive(Args)]
pub struct ServeArgs {
    /// Address the HTTP server listens on
    #[arg(long, env = "CONVERTISSEUR_BIND_ADDR", default_value_t = default_bind_addr())]
    pub bind_addr: SocketAddr,

    /// PostgreSQL connection string
    #[arg(long, env = "DATABASE_URL", hide_env_values = true)]
    pub database_url: String,

    /// Maximum number of connections in the database pool
    #[arg(long, env = "CONVERTISSEUR_DB_MAX_CONNECTIONS", default_value_t = 10)]
    pub db_max_connections: u32,

    /// Directory holding the built webapp, served for every path outside the
    /// API
    #[arg(
        long,
        env = "CONVERTISSEUR_WEBAPP_DIR",
        default_value = "/usr/share/convertisseur/webapp"
    )]
    pub webapp_dir: PathBuf,

    /// Username of the super admin account, created or updated on every
    /// start-up; any other super admin is demoted to admin
    #[arg(long, env = "CONVERTISSEUR_SUPER_ADMIN_USERNAME")]
    pub super_admin_username: String,

    /// Password of the super admin account (at least 12 characters), applied
    /// on every start-up
    #[arg(
        long,
        env = "CONVERTISSEUR_SUPER_ADMIN_PASSWORD",
        hide_env_values = true
    )]
    pub super_admin_password: String,
}

#[derive(Args)]
pub struct HealthcheckArgs {
    /// Address the server listens on; the probe connects to its port on
    /// 127.0.0.1
    #[arg(long, env = "CONVERTISSEUR_BIND_ADDR", default_value_t = default_bind_addr())]
    pub bind_addr: SocketAddr,
}

fn default_bind_addr() -> SocketAddr {
    SocketAddr::from(([0, 0, 0, 0], 3000))
}
