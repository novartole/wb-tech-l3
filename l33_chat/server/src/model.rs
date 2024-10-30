use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Room {
    pub id: Option<String>,
    pub title: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct User {
    #[serde(skip)]
    pub id: Option<String>,
    pub username: String,
}

#[derive(Clone, Deserialize, Validate)]
pub struct Message {
    #[serde(skip_deserializing)]
    pub id: Option<String>,
    #[validate(length(min = 7))]
    pub room_id: String,
    #[validate(length(min = 7))]
    pub user_id: String,
    pub text: String,
}
