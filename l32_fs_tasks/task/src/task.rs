use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub complete_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskOutput {
    Value(Option<String>),
    Error(String),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CompletedTask {
    pub id: Uuid,
    pub task: Task,
    pub output: TaskOutput,
    pub completed_at: DateTime<Utc>,
}

