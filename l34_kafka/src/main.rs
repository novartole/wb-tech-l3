mod bus;
mod cli;
mod error;
mod handler;
mod model;
mod repo;
mod state;

use anyhow::{Context, Result};
use axum::{
    routing::{delete, post, put},
    Router,
};
use bus::{kafka::KafkaMsgProducer, NotifyBus};
use clap::Parser;
use cli::Cli;
use handler::{
    create_product, create_user, delete_product, delete_user, update_product, update_user,
};
use model::{BusMessage, ProductDiff, UserDiff};
use repo::{
    postgres::{PgEventListener, PgRepo},
    ListenEvent,
};
use state::AppState;
use std::{env, net::SocketAddr};
use tokio::{
    net::TcpListener,
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
};
use tracing::{error, info};
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
        bus_params,
    } = Cli::try_parse().context("Parsing args")?;

    let (msg_tx, msg_rx) = mpsc::unbounded_channel();
    let event_listener = PgEventListener::new(&db_params);
    let kafka_producer = KafkaMsgProducer::new(&bus_params)?;
    listen_db_events(event_listener, msg_tx).await?;
    notify_msg_bus(kafka_producer, msg_rx);

    let tcp_listener = {
        let addr = SocketAddr::from((ip, port));
        TcpListener::bind(addr).await.context("Creating listener")?
    };
    let app = {
        let pg_repo = PgRepo::try_new(&db_params).await?;
        Router::new()
            .route("/users", post(create_user))
            .route("/users/:id", put(update_user))
            .route("/users/:id", delete(delete_user))
            .route("/products", post(create_product))
            .route("/products/:id", put(update_product))
            .route("/products/:id", delete(delete_product))
            .with_state(AppState { repo: pg_repo })
    };
    info!("listening on {}", tcp_listener.local_addr().unwrap());
    axum::serve(tcp_listener, app)
        .await
        .context("Running service")
}

fn notify_msg_bus(notifier: impl NotifyBus, msg_rx: UnboundedReceiver<BusMessage>) {
    notifier.redirect(msg_rx);
}

async fn listen_db_events(
    listener: impl ListenEvent,
    msg_tx: UnboundedSender<BusMessage>,
) -> Result<()> {
    let on_user_change = {
        let tx_ = msg_tx.clone();
        move |val: UserDiff| {
            if let Err(why) = tx_.send(BusMessage::User(val)) {
                error!("failed sending message to kafka producer: {:?}", why);
            }
        }
    };

    let on_product_change = move |val: ProductDiff| {
        if let Err(why) = msg_tx.send(BusMessage::Product(val)) {
            error!("failed sending message to kafka producer: {:?}", why);
        }
    };

    listener
        .listen("users_change", on_user_change)
        .await
        .context("Start listening 'users_change' notifications")?
        .listen("products_change", on_product_change)
        .await
        .context("Start listening 'products_change' notifications")?;

    Ok(())
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
