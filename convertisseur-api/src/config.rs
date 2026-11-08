use std::{net::SocketAddr, path::PathBuf};

use axum::http::HeaderName;
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(version, about = "Convertisseur HTTP API and web server")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// `tracing` filter directive (e.g. `info`, `convertisseur_api=debug,sqlx=warn`)
    ///
    /// Syntax: <https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives>
    #[arg(long, env = "LOG_FILTER", default_value = "info", global = true)]
    pub log_filter: String,

    /// Log output format
    #[arg(
        long,
        env = "LOG_FORMAT",
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
    #[arg(long, env = "BIND_ADDR", default_value_t = default_bind_addr())]
    pub bind_addr: SocketAddr,

    /// PostgreSQL connection string
    #[arg(long, env = "DATABASE_URL", hide_env_values = true)]
    pub database_url: String,

    /// Maximum number of connections in the database pool
    #[arg(long, env = "DB_MAX_CONNECTIONS", default_value_t = 10)]
    pub db_max_connections: u32,

    /// Directory holding the built webapp, served for every path outside the
    /// API
    #[arg(
        long,
        env = "WEBAPP_DIR",
        default_value = "/usr/share/convertisseur/webapp"
    )]
    pub webapp_dir: PathBuf,

    /// Username of the super admin account, created or updated on every
    /// start-up; any other super admin is demoted to admin
    #[arg(long, env = "SUPER_ADMIN_USERNAME")]
    pub super_admin_username: String,

    /// Password of the super admin account (at least 12 characters), applied
    /// on every start-up
    #[arg(long, env = "SUPER_ADMIN_PASSWORD", hide_env_values = true)]
    pub super_admin_password: String,

    /// Mark the session cookie `Secure`, so that browsers only send it over
    /// HTTPS; turn it off only to serve plain HTTP in development
    #[arg(long, env = "COOKIE_SECURE", default_value_t = true, action = ArgAction::Set)]
    pub cookie_secure: bool,

    /// Header in which the reverse proxy in front of the server passes the
    /// client's IP (e.g. `X-Forwarded-For`, `X-Real-IP`); its last address is
    /// used. Unset, the client is the TCP peer. Set it only behind a proxy
    /// that overwrites the header, since clients can forge it
    #[arg(long, env = "CLIENT_IP_HEADER")]
    pub client_ip_header: Option<HeaderName>,
}

#[derive(Args)]
pub struct HealthcheckArgs {
    /// Address the server listens on; the probe connects to its port on
    /// 127.0.0.1
    #[arg(long, env = "BIND_ADDR", default_value_t = default_bind_addr())]
    pub bind_addr: SocketAddr,
}

fn default_bind_addr() -> SocketAddr {
    SocketAddr::from(([0, 0, 0, 0], 3000))
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;

    fn env_names(command: &clap::Command) -> Vec<(String, String)> {
        command
            .get_arguments()
            .filter_map(|arg| {
                let env = arg.get_env()?.to_str()?.to_owned();
                Some((arg.get_id().to_string(), env))
            })
            .collect()
    }

    #[test]
    fn the_cli_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn serve_reads_bare_environment_variables() {
        let cli = Cli::command();
        let serve = cli.find_subcommand("serve").unwrap();
        let mut names = env_names(&cli);
        names.extend(env_names(serve));
        names.sort();
        assert_eq!(
            names,
            [
                ("bind_addr", "BIND_ADDR"),
                ("client_ip_header", "CLIENT_IP_HEADER"),
                ("cookie_secure", "COOKIE_SECURE"),
                ("database_url", "DATABASE_URL"),
                ("db_max_connections", "DB_MAX_CONNECTIONS"),
                ("log_filter", "LOG_FILTER"),
                ("log_format", "LOG_FORMAT"),
                ("super_admin_password", "SUPER_ADMIN_PASSWORD"),
                ("super_admin_username", "SUPER_ADMIN_USERNAME"),
                ("webapp_dir", "WEBAPP_DIR"),
            ]
            .map(|(id, env)| (id.to_owned(), env.to_owned()))
        );
    }
}
