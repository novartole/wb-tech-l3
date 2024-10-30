use crate::{
    error::Error,
    model::{Message, Room, User},
    state::StoreChat,
};
use anyhow::{anyhow, Context};
use dashmap::DashMap;
use std::{collections::HashSet, sync::Arc};
use tracing::debug;
use uuid::Uuid;

#[derive(Debug)]
struct ImrRoom {
    title: String,
    users: HashSet<String>,
}

#[derive(Debug)]
struct ImrUser {
    username: String,
}

#[allow(dead_code)]
#[derive(Debug)]
struct ImrMessage {
    room_id: String,
    user_id: String,
    text: String,
}

#[derive(Clone, Default)]
pub struct InMemoryRepo {
    rooms: Arc<DashMap<String, ImrRoom>>,
    users: Arc<DashMap<String, ImrUser>>,
    messages: Arc<DashMap<String, ImrMessage>>,
}

impl InMemoryRepo {
    pub fn new() -> Self {
        let rooms = Arc::new(DashMap::new());
        rooms.insert(
            String::from("67e5504"),
            ImrRoom {
                title: String::from("room 1"),
                users: Default::default(),
            },
        );
        rooms.insert(
            String::from("67e5505"),
            ImrRoom {
                title: String::from("room 2"),
                users: Default::default(),
            },
        );

        let users = Arc::new(DashMap::new());
        users.insert(
            String::from("67e5504"),
            ImrUser {
                username: String::from("user 1"),
            },
        );
        users.insert(
            String::from("67e5505"),
            ImrUser {
                username: String::from("user 2"),
            },
        );

        Self {
            rooms,
            users,
            ..Default::default()
        }
    }

    fn generate_key() -> String {
        let mut res = Uuid::new_v4().as_simple().to_string();
        res.truncate(7);
        res
    }
}

impl StoreChat for InMemoryRepo {
    async fn add_user_to_room(&self, user_id: &str, room_id: &str) -> Result<bool, Error> {
        let user_id = self
            .get_user(user_id)
            .await?
            .id
            .context("User ID cannot be None")?;

        debug!(rooms = ?self.rooms, "rooms before");
        if !self
            .rooms
            .get_mut(room_id)
            .ok_or(Error::NotFound("Room"))?
            .users
            .insert(user_id)
        {
            return Ok(false);
        }
        debug!(rooms = ?self.rooms, "rooms after");

        Ok(true)
    }

    async fn remove_user_from_room(&self, user_id: &str, room_id: &str) -> Result<bool, Error> {
        debug!(rooms = ?self.rooms, "rooms before");
        if !self
            .rooms
            .get_mut(room_id)
            .ok_or(Error::NotFound("Room"))?
            .users
            .remove(user_id)
        {
            return Ok(true);
        }
        debug!(rooms = ?self.rooms, "rooms after");

        Ok(false)
    }

    async fn is_user_in_room(&self, user_id: &str, room_id: &str) -> Result<bool, Error> {
        let user_id = self
            .get_user(user_id)
            .await?
            .id
            .context("User ID cannot be None")?;

        Ok(self
            .rooms
            .get(room_id)
            .ok_or(Error::NotFound("Room"))?
            .users
            .contains(&user_id))
    }

    async fn create_room(&self, Room { id, title }: Room) -> Result<Room, Error> {
        let room_id = id.unwrap_or(Self::generate_key());

        if self.rooms.contains_key(&room_id) {
            return Err(Error::Other(anyhow!("attempt to insert existed room")));
        }

        let room = ImrRoom {
            title: title.clone(),
            users: HashSet::new(),
        };

        debug!(rooms = ?self.rooms, "rooms before");
        self.rooms.insert(room_id.clone(), room);
        debug!(rooms = ?self.rooms, "rooms after");

        Ok(Room {
            id: Some(room_id),
            title,
        })
    }

    async fn get_room(&self, room_id: &str) -> Result<Room, Error> {
        self.rooms
            .get(room_id)
            .ok_or(Error::NotFound("Room"))
            .map(|room| Room {
                id: Some(room.key().to_string()),
                title: room.title.clone(),
            })
    }

    async fn create_user(&self, User { id, username }: User) -> Result<User, Error> {
        let user_id = id.unwrap_or(Self::generate_key());

        if self.rooms.contains_key(&user_id) {
            return Err(Error::Other(anyhow!("attempt to insert existed user")));
        }

        let room = ImrUser {
            username: username.clone(),
        };

        debug!(users = ?self.users, "users before");
        self.users.insert(user_id.clone(), room);
        debug!(users = ?self.users, "users after");

        Ok(User {
            id: Some(user_id),
            username,
        })
    }

    async fn get_user(&self, user_id: &str) -> Result<User, Error> {
        self.users
            .get(user_id)
            .ok_or(Error::NotFound("User"))
            .map(|user| User {
                id: Some(user.key().to_string()),
                username: user.username.clone(),
            })
    }

    async fn create_message(
        &self,
        Message {
            id,
            room_id,
            user_id,
            text,
        }: Message,
    ) -> Result<Message, Error> {
        let msg_id = id.unwrap_or(Self::generate_key());

        if self.messages.contains_key(&msg_id) {
            return Err(Error::Other(anyhow!("attempt to insert existed message")));
        }

        let msg = ImrMessage {
            room_id: room_id.clone(),
            user_id: user_id.clone(),
            text: text.clone(),
        };

        debug!(messages = ?self.messages, "messages before");
        self.messages.insert(msg_id.clone(), msg);
        debug!(messages = ?self.messages, "messages after");

        Ok(Message {
            id: Some(msg_id),
            room_id,
            user_id,
            text,
        })
    }
}
