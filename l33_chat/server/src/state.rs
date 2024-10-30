use crate::{
    error::Error,
    model::{Message, Room, User},
    notifier::Notifier,
};
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState<R>
where
    R: StoreChat,
{
    pub repo: R,
    pub pool: Arc<DashMap<String, Notifier>>,
}

pub trait StoreChat: Clone {
    async fn add_user_to_room(&self, user_id: &str, room_id: &str) -> Result<bool, Error>;
    async fn remove_user_from_room(&self, user_id: &str, room_id: &str) -> Result<bool, Error>;
    async fn is_user_in_room(&self, user_id: &str, room_id: &str) -> Result<bool, Error>;

    async fn create_room(&self, room: Room) -> Result<Room, Error>;
    async fn get_room(&self, room_id: &str) -> Result<Room, Error>;

    async fn create_user(&self, user: User) -> Result<User, Error>;
    async fn get_user(&self, user_id: &str) -> Result<User, Error>;

    async fn create_message(&self, message: Message) -> Result<Message, Error>;
}
