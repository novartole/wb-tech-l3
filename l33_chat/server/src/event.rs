use crate::model::User;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
enum EventType {
    Join,
    Leave,
    #[serde(rename(serialize = "message"))]
    Msg(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct Event {
    #[serde(rename(serialize = "type"))]
    ty: EventType,
    user: User,
}

impl Event {
    pub fn is_user_leaving(&self, user: &User) -> bool {
        matches!(self.ty, EventType::Leave if self.user.id == user.id)
    }

    pub fn join(user: User) -> Self {
        Self {
            user,
            ty: EventType::Join,
        }
    }

    pub fn leave(user: User) -> Self {
        Self {
            user,
            ty: EventType::Leave,
        }
    }

    pub fn message(user: User, text: String) -> Self {
        Self {
            user,
            ty: EventType::Msg(text),
        }
    }
}
