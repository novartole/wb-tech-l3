pub mod redis;

use crate::{
    error::Error,
    model::{Event, Id, Subscription},
};
use tokio::sync::broadcast;

pub trait Store: Clone {
    async fn create_event(&self, event: Event) -> Result<(), Error>;
    async fn get_events(&self, user_id: &Id) -> Result<Vec<Event>, Error>;
    async fn create_subscription(&self, sub: Subscription) -> Result<(), Error>;
}

pub trait ListenEvents: Clone {
    fn subscribe(&self) -> broadcast::Receiver<Event>;
}
