use anyhow::Context;
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use chrono::{DateTime, Utc};
use clap::{value_parser, Parser};
use serde::Deserialize;
use std::{
    env,
    net::{Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::Arc,
};
use task::Task;
use thiserror::Error;
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    net::TcpListener,
    sync::mpsc::{self, UnboundedSender},
};
use tower_http::trace::TraceLayer;
use tracing::{debug, error, info, trace};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    setup_tracing();

    if let Err(why) = run().await {
        error!("failed to serve: {}", why);
    }
}

fn setup_tracing() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    // Turn on error backtrace by default.
    // FYI:
    // - if you want panics and errors to both have backtraces, set RUST_BACKTRACE=1,
    // - If you want only errors to have backtraces, set RUST_LIB_BACKTRACE=1,
    // - if you want only panics to have backtraces, set RUST_BACKTRACE=1 and RUST_LIB_BACKTRACE=0.
    if env::var("RUST_LIB_BACKTRACE").is_err() {
        env::set_var("RUST_LIB_BACKTRACE", "1");
    }

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .pretty()
        .init();
}

#[derive(Debug, Error)]
enum Error {
    #[error("Input validation error: {0}")]
    BadRequest(&'static str),

    #[error("Please try later")]
    QueueWorker(#[from] mpsc::error::SendError<Task>),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Error::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            Error::QueueWorker(ref err) => {
                error!("failed sending task to worker: {:?}", err);
                (StatusCode::SERVICE_UNAVAILABLE, self.to_string())
            }
        }
        .into_response()
    }
}

#[derive(Parser)]
struct Cli {
    /// Listening IP
    #[clap(short, long, default_value = "0.0.0.0", env = "WBTECH_L32_CREATOR_IP")]
    ip: Ipv4Addr,

    /// Listening port
    #[clap(
        short,
        long,
        value_parser = value_parser!(u16).range(1..),
        default_value_t = 3000,
        env = "WBTECH_L32_CREATOR_PORT"
    )]
    port: u16,

    /// Folder to store created tasks.
    #[clap(long, default_value = "output", env = "WBTECH_L32_CREATOR_OUTPUT")]
    output: PathBuf,
}

async fn run() -> Result<(), anyhow::Error> {
    let Cli { ip, port, output } = Cli::try_parse()?;

    let worker_tx = {
        let (tx, mut rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            while let Some(task) = rx.recv().await {
                trace!(?task, "worker: got a new task");
                if let Err(why) = save(task, &output).await {
                    error!("failed to save task: {:?}", why);
                }
            }
        });
        tx
    };

    let listener = {
        let addr = SocketAddr::from((ip, port));
        TcpListener::bind(addr).await.context("Creating listener")?
    };

    let app = Router::new()
        .route("/create_task", post(create_task))
        .layer(TraceLayer::new_for_http())
        .with_state(AppState::new(worker_tx));

    info!("start listening on {:?}:{}", ip, port);
    axum::serve(listener, app).await.context("Running server")
}

async fn save(task: Task, path: impl AsRef<Path>) -> Result<(), anyhow::Error> {
    debug!(?task, "saving task");

    let mut file = {
        let filename = task.id.to_string();
        let mut path = path.as_ref().join(&filename);
        path.set_extension("json");
        File::create_new(&path)
            .await
            .inspect(|_| trace!(file = path.to_string_lossy().as_ref(), "task file created"))
            .context("Creating task file")?
    };

    let content = serde_json::to_vec(&task)
        .inspect(|bytes| trace!(bytes = bytes.len(), "task serialized"))
        .context("Serializing task")?;

    file.write_all(&content)
        .await
        .inspect(|_| trace!("written {} bytes", content.len()))
        .context("Saving task into file")
}

#[derive(Deserialize)]
struct TaskDto {
    pub id: Option<Uuid>,
    pub title: String,
    pub description: String,
    pub created_at: Option<DateTime<Utc>>,
    pub complete_until: Option<DateTime<Utc>>,
}

impl From<TaskDto> for Task {
    fn from(task_dto: TaskDto) -> Self {
        Self {
            id: task_dto.id.unwrap_or(Uuid::new_v4()),
            title: task_dto.title,
            description: task_dto.description,
            created_at: task_dto.created_at.unwrap_or(Utc::now()),
            complete_until: task_dto.complete_until,
        }
    }
}

#[axum::debug_handler]
async fn create_task(
    State(AppState { worker_tx }): State<AppState>,
    Json(paylaod): Json<TaskDto>,
) -> Result<StatusCode, Error> {
    let task = Task::from(paylaod);
    debug!(?task);

    if task
        .complete_until
        .is_some_and(|time| time < task.created_at)
    {
        return Err(Error::BadRequest(
            "complete_until happens earlier than created_at",
        ));
    }

    worker_tx
        .send(task)
        .inspect(|_| trace!("task was sent to worker"))?;

    Ok(StatusCode::ACCEPTED)
}

#[derive(Clone)]
struct AppState {
    worker_tx: Arc<UnboundedSender<Task>>,
}

impl AppState {
    fn new(worker_tx: UnboundedSender<Task>) -> Self {
        Self {
            worker_tx: Arc::new(worker_tx),
        }
    }
}
