use crate::event::Event;
use tokio::sync::broadcast::{self, error::SendError, Receiver, Sender};

pub struct Notifier {
    tx: Sender<Event>,
    _rx: Receiver<Event>,
}

impl Notifier {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(1);
        Self { tx, _rx }
    }

    pub fn subscribe(&self) -> Receiver<Event> {
        self.tx.subscribe()
    }

    pub fn send(&self, event: Event) -> Result<(), SendError<Event>> {
        self.tx.send(event)?;
        Ok(())
    }
}
