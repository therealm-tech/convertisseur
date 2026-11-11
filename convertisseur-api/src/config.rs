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
    /// Apply the database migrations, set up the super admin and the object
    /// store, bring the content of the built-in content files into the
    /// database, then serve the API and the webapp
    Serve(Box<ServeArgs>),
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

    #[command(flatten)]
    pub storage: StorageArgs,
}

/// The S3-compatible object store holding uploaded media.
#[derive(Args, Clone)]
pub struct StorageArgs {
    /// URL of the S3-compatible object store, e.g.
    /// `https://s3.eu-west-3.amazonaws.com`
    #[arg(long = "s3-endpoint", env = "S3_ENDPOINT")]
    pub endpoint: String,

    /// Region of the bucket; a store with no regions accepts any
    #[arg(long = "s3-region", env = "S3_REGION", default_value = "us-east-1")]
    pub region: String,

    /// Bucket holding the media; the server creates it on start-up when it
    /// does not exist
    #[arg(long = "s3-bucket", env = "S3_BUCKET")]
    pub bucket: String,

    /// Access key ID of the object store's credentials
    #[arg(long = "s3-access-key-id", env = "S3_ACCESS_KEY_ID")]
    pub access_key_id: String,

    /// Secret access key of the object store's credentials
    #[arg(
        long = "s3-secret-access-key",
        env = "S3_SECRET_ACCESS_KEY",
        hide_env_values = true
    )]
    pub secret_access_key: String,

    /// Name the bucket in the URL's path (`endpoint/bucket/key`) rather than
    /// in its host name (`bucket.endpoint/key`); most self-hosted stores
    /// need it
    #[arg(
        long = "s3-force-path-style",
        env = "S3_FORCE_PATH_STYLE",
        default_value_t = false,
        action = ArgAction::Set
    )]
    pub force_path_style: bool,
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
                ("access_key_id", "S3_ACCESS_KEY_ID"),
                ("bind_addr", "BIND_ADDR"),
                ("bucket", "S3_BUCKET"),
                ("client_ip_header", "CLIENT_IP_HEADER"),
                ("cookie_secure", "COOKIE_SECURE"),
                ("database_url", "DATABASE_URL"),
                ("db_max_connections", "DB_MAX_CONNECTIONS"),
                ("endpoint", "S3_ENDPOINT"),
                ("force_path_style", "S3_FORCE_PATH_STYLE"),
                ("log_filter", "LOG_FILTER"),
                ("log_format", "LOG_FORMAT"),
                ("region", "S3_REGION"),
                ("secret_access_key", "S3_SECRET_ACCESS_KEY"),
                ("super_admin_password", "SUPER_ADMIN_PASSWORD"),
                ("super_admin_username", "SUPER_ADMIN_USERNAME"),
                ("webapp_dir", "WEBAPP_DIR"),
            ]
            .map(|(id, env)| (id.to_owned(), env.to_owned()))
        );
    }
}
