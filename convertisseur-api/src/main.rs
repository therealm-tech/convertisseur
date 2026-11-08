use std::{net::SocketAddr, time::Duration};

use anyhow::{Context, bail};
use clap::Parser;
use convertisseur_api::{
    AppState,
    config::{Cli, Command, HealthcheckArgs, LogFormat, ServeArgs},
};
use sqlx::postgres::PgPoolOptions;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    // `EnvFilter::new` silently drops a directive it cannot parse.
    let fmt = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_new(&cli.log_filter)?)
        .with_writer(std::io::stderr);
    match cli.log_format {
        LogFormat::Text => fmt.init(),
        LogFormat::Json => fmt.json().init(),
    }

    match cli.command {
        Command::Serve(args) => serve(args).await,
        Command::Healthcheck(args) => healthcheck(&args).await,
    }
}

async fn serve(args: ServeArgs) -> anyhow::Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(args.db_max_connections)
        .connect(&args.database_url)
        .await
        .context("failed to connect to the database")?;
    convertisseur_api::MIGRATOR
        .run(&pool)
        .await
        .context("failed to apply the database migrations")?;
    tracing::info!("database migrations applied");
    convertisseur_api::ensure_super_admin(
        &pool,
        &args.super_admin_username,
        args.super_admin_password,
    )
    .await
    .context("failed to set up the super admin account")?;

    let app = convertisseur_api::app(AppState::new(pool.clone()), &args.webapp_dir);
    let listener = TcpListener::bind(args.bind_addr)
        .await
        .with_context(|| format!("failed to bind {}", args.bind_addr))?;
    tracing::info!(addr = %args.bind_addr, "listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    pool.close().await;
    tracing::info!("server stopped");
    Ok(())
}

async fn shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};
    let mut interrupt = signal(SignalKind::interrupt()).expect("SIGINT handler");
    let mut terminate = signal(SignalKind::terminate()).expect("SIGTERM handler");
    tokio::select! {
        _ = interrupt.recv() => tracing::info!("received SIGINT, shutting down"),
        _ = terminate.recv() => tracing::info!("received SIGTERM, shutting down"),
    }
}

/// The runtime image has no shell and no HTTP client, so the container's
/// health check goes through the binary itself.
async fn healthcheck(args: &HealthcheckArgs) -> anyhow::Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], args.bind_addr.port()));
    let probe = async {
        let mut stream = TcpStream::connect(addr).await?;
        stream
            .write_all(b"GET /healthz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await?;
        anyhow::Ok(response)
    };
    let response = tokio::time::timeout(Duration::from_secs(3), probe)
        .await
        .context("health probe timed out")??;
    let status_line = response.split(|&b| b == b'\r').next().unwrap_or_default();
    tracing::debug!(%addr, status = %String::from_utf8_lossy(status_line), "health probe answered");
    if !status_line.starts_with(b"HTTP/1.1 200") {
        bail!("unhealthy: {}", String::from_utf8_lossy(status_line));
    }
    Ok(())
}
