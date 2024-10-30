use bitflags::bitflags;
use chrono::{DateTime, Utc};
use redis_macros::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};

pub type Id = i32;

bitflags! {
    #[derive(Deserialize, Serialize, ToRedisArgs, FromRedisValue, Clone, Copy, Debug)]
    // make serde not to wrap it in newtype
    #[serde(transparent)]
    pub struct EventType: u8 {
        const ET1 = 1;
        const ET2 = 2;
        const ET3 = 4;
    }
}

#[derive(Deserialize, Serialize, ToRedisArgs, FromRedisValue, Debug, Clone)]
pub struct Event {
    pub id: Option<Id>,
    #[serde(rename = "type")]
    pub ty: EventType,
    pub ts: DateTime<Utc>,
    /// any associated data
    pub data: Option<String>,
}

#[derive(Deserialize, Clone, Copy)]
pub struct Subscription {
    pub user_id: Id,
    pub event_ty: EventType,
}
