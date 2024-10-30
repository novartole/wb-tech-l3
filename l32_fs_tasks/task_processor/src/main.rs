use anyhow::{Context, Error, Result};
use chrono::Utc;
use clap::Parser;
use futures::StreamExt;
use notifier::try_watch;
use std::{
    env,
    path::{Path, PathBuf},
};
use task::{CompletedTask, Task, TaskOutput};
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
};
use tracing::{debug, error, trace};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[derive(Parser)]
struct Cli {
    /// Folder of created tasks.
    #[clap(
        short,
        long,
        default_value = "input",
        env = "WBTECH_L32_PROCESSOR_INPUT"
    )]
    input: PathBuf,

    /// Folder to store handled tasks.
    #[clap(
        short,
        long,
        default_value = "output",
        env = "WBTECH_L32_PROCESSOR_OUTPUT"
    )]
    output: PathBuf,
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
        .pretty()
        .init();
}

async fn run() -> Result<()> {
    let Cli { input, output } = Cli::try_parse().context("Parsing args")?;

    let mut notifier = try_watch(&input).await.context("Creating notifier")?;

    while let Some(try_path) = notifier.next().await {
        let task_file = try_path.context("Getting path of task file")?;

        trace!(
            file = task_file.to_string_lossy().as_ref(),
            "proceeding task file"
        );

        if let Err(why) = proceed(task_file, &output).await {
            error!("failed to proceed task file: {:?}", why);
        }
    }

    Ok(())
}
async fn proceed(from: impl AsRef<Path>, into: impl AsRef<Path>) -> Result<()> {
    let task = read(from).await?;
    let comp_task = execute(task).await?;
    save(comp_task, into).await
}

async fn read(path: impl AsRef<Path>) -> Result<Task> {
    trace!(
        file = path.as_ref().to_string_lossy().as_ref(),
        "reading task file"
    );

    let content = fs::read(path)
        .await
        .inspect(|bytes| trace!("read {} bytes", bytes.len()))
        .context("Reading task file")?;

    let task = serde_json::from_slice(&content)
        .inspect(|task| debug!(?task, "task extracted"))
        .context("Getting task from file")?;

    Ok(task)
}

async fn execute(task: Task) -> Result<CompletedTask, Error> {
    debug!(?task, "executing task");

    return executing(task)
        .await
        .inspect(|comp_task| debug!(?comp_task, "task executed"))
        .context("Executing task");

    async fn executing(task: Task) -> Result<CompletedTask> {
        Ok(CompletedTask {
            id: Uuid::new_v4(),
            task,
            output: TaskOutput::Value(None),
            completed_at: Utc::now(),
        })
    }
}

async fn save(comp_task: CompletedTask, path: impl AsRef<Path>) -> Result<()> {
    debug!(?comp_task, "saving comp task");

    let mut file = {
        let filename = comp_task.id.to_string();
        let mut path = path.as_ref().join(&filename);
        path.set_extension("json");
        File::create_new(&path)
            .await
            .inspect(|_| {
                trace!(
                    file = path.to_string_lossy().as_ref(),
                    "comp task file created"
                )
            })
            .context("Creating comp task file")?
    };

    let content = serde_json::to_vec(&comp_task)
        .inspect(|bytes| trace!(bytes = bytes.len(), "comp task serialized"))
        .context("Serialazing comp task")?;

    file.write_all(&content)
        .await
        .inspect(|_| trace!("written {} bytes", content.len()))
        .context("Saving comp task into file")?;

    Ok(())
}
