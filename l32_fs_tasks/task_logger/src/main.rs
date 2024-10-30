use anyhow::{Context, Result};
use clap::Parser;
use futures::StreamExt;
use notifier::try_watch;
use std::{
    env,
    path::{Path, PathBuf},
};
use task::{CompletedTask, TaskOutput};
use tokio::fs::{self};
use tracing::{debug, error, trace};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
struct Cli {
    /// Folder of completed tasks.
    #[clap(short, long, default_value = "input", env = "WBTECH_L32_LOGGER_INPUT")]
    input: PathBuf,

    /// Folder to store logging information.
    #[clap(
        short,
        long,
        default_value = "output",
        env = "WBTECH_L32_LOGGER_OUTPUT"
    )]
    output: PathBuf,

    /// Logging journal file name.
    #[clap(
        short,
        long,
        default_value = "processed-tasks.log",
        env = "WBTECH_L32_LOGGER_FILE"
    )]
    file: PathBuf,
}

#[tokio::main]
async fn main() {
    setup_tracing();

    if let Err(why) = run().await {
        error!("fatal error: {:?}", why);
    }
}

fn setup_tracing() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    if env::var("RUST_LIB_BACKTRACE").is_err() {
        env::set_var("RUST_LIB_BACKTRACE", "1");
    }

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .compact()
        .init();
}

async fn run() -> Result<()> {
    let Cli {
        input,
        output,
        file,
    } = Cli::try_parse().context("Parsing args")?;

    let (log_tx, log_rx) = std::sync::mpsc::channel::<CompletedTask>();

    let log_worker = tokio::task::spawn_blocking(move || {
        let appender = tracing_appender::rolling::never(output, file);
        let subscriber = tracing_subscriber::fmt()
            .with_writer(appender)
            .with_ansi(false)
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            while let Some(task) = log_rx.iter().next() {
                match task.output {
                    TaskOutput::Value(_) => tracing::info!(?task),
                    TaskOutput::Error(_) => tracing::error!(?task),
                }
            }
        });
    });

    let mut notifier = try_watch(&input).await.context("Creating notifier")?;

    let res = loop {
        match notifier.next().await {
            Some(try_path) => {
                let task_file = match try_path.context("Getting task file") {
                    Err(e) => break Err(e),
                    Ok(file) => file,
                };

                let task = match read(task_file).await.context("Getting task from file") {
                    Err(e) => break Err(e),
                    Ok(task) => task,
                };

                if let Err(e) = log_tx.send(task).context("Send to log worker") {
                    break Err(e);
                }
            }
            None => break Ok(()),
        }
    };

    if let Err(why) = log_worker.await {
        error!("failed to wait for cancellation of log worker: {:?}", why);
    }

    res
}

async fn read(path: impl AsRef<Path>) -> Result<CompletedTask> {
    trace!(
        file = path.as_ref().to_string_lossy().as_ref(),
        "reading task file"
    );

    let content = fs::read(path)
        .await
        .inspect(|bytes| trace!("read {} bytes", bytes.len()))
        .context("Reading task file")?;

    let comp_task = serde_json::from_slice(&content)
        .inspect(|task| debug!(?task, "comp task extracted"))
        .context("Getting task from file")?;

    Ok(comp_task)
}
