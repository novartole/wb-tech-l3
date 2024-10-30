mod cli;
mod dto;
mod error;
mod handler;
mod model;
mod repo;
mod state;

use anyhow::{Context, Result};
use axum::{
    routing::{get, post},
    Router,
};
use clap::Parser;
use cli::Cli;
use handler::{create_event, get_events, subscribe};
use repo::redis::{RedisEventListener, RedisRepo, EVENT_CHANNEL_NAME};
use state::{AppState, SubManager};
use std::{env, net::SocketAddr};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    setup_tracing();

    if let Err(why) = run().await {
        eprintln!("fatal error: {:?}", why);
    }
}

async fn run() -> Result<()> {
    let Cli {
        ip,
        port,
        db_params,
    } = Cli::try_parse().context("Parsing args")?;

    let repo = RedisRepo::try_new(&db_params).await?;
    let event_listener = RedisEventListener::new(&db_params, EVENT_CHANNEL_NAME).await?;
    let sub_manager = SubManager::default();

    let tcp_listener = {
        let addr = SocketAddr::from((ip, port));
        TcpListener::bind(addr).await.context("Creating listener")?
    };

    let app = Router::new()
        .route("/events", post(create_event))
        .route("/subscribe", post(subscribe))
        .route("/events/:user_id", get(get_events))
        .with_state(AppState {
            repo,
            event_listener,
            sub_manager,
        })
        .layer(TraceLayer::new_for_http());

    info!("listening on {}", tcp_listener.local_addr().unwrap());
    axum::serve(tcp_listener, app)
        .await
        .context("Running service")
}

fn setup_tracing() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "trace");
    }

    if env::var("RUST_LIB_BACKTRACE").is_err() {
        env::set_var("RUST_LIB_BACKTRACE", "1");
    }

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .pretty()
        .init();
}
