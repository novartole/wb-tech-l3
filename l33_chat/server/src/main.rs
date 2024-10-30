mod cli;
mod error;
mod event;
mod handler;
mod model;
mod notifier;
mod repo;
mod state;

use anyhow::{Context, Result};
use axum::{
    routing::{get, post},
    Router,
};
use clap::Parser;
use cli::Cli;
use handler::{create_room, create_user, join_room, leave_room, send_message, ws_messages};
use repo::InMemoryRepo;
use state::AppState;
use std::{env, net::SocketAddr};
use tokio::net::TcpListener;
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
    let Cli { ip, port } = Cli::try_parse().context("Parsing args")?;

    let listener = {
        let addr = SocketAddr::from((ip, port));
        TcpListener::bind(addr).await.context("Creating listener")?
    };

    let app = Router::new()
        .route("/join", post(join_room))
        .route("/leave", post(leave_room))
        .route("/send", post(send_message))
        .route("/messages", get(ws_messages))
        .route("/create_user", post(create_user))
        .route("/create_room", post(create_room))
        .with_state(AppState {
            repo: InMemoryRepo::new(),
            pool: Default::default(),
        });

    info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await.context("Running service")
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
