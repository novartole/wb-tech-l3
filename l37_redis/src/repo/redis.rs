use super::{ListenEvents, Store};
use crate::{
    error::Error,
    model::{Event, EventType, Id, Subscription},
};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use redis::{AsyncCommands, AsyncIter, IntoConnectionInfo};
use tokio::{select, sync::broadcast};
use tokio_stream::StreamExt;
use tracing::debug;

pub const EVENT_CHANNEL_NAME: &str = "EVENT";

fn get_event_id_key() -> &'static str {
    "event_id"
}

fn get_event_key() -> &'static str {
    "event"
}

fn get_event_field(event_id: &Id, event_ty: EventType) -> String {
    format!(
        "{}:{}",
        event_id,
        format!("{:08b}", event_ty.bits())
            .chars()
            .rev()
            .collect::<String>()
    )
}

fn get_event_filed_pattern(event_ty: EventType) -> String {
    "*:".to_owned()
        + &format!("{:08b}", event_ty.bits().swap_bytes())
            .chars()
            .rev()
            .map(|ch| if ch == '1' { '*' } else { ch })
            .collect::<String>()
}

fn get_user_key(user_id: &Id) -> String {
    format!("user{}", user_id)
}

#[derive(Clone)]
pub struct RedisRepo {
    pool: Pool<RedisConnectionManager>,
}

impl RedisRepo {
    pub async fn try_new(params: &str) -> Result<Self, Error> {
        let config = params.into_connection_info()?;
        let manager = RedisConnectionManager::new(config)?;
        let pool = Pool::builder().build(manager).await?;

        let instance = Self { pool };
        instance.init().await?;
        Ok(instance)
    }

    async fn init(&self) -> Result<(), Error> {
        Ok(self.pool.get().await?.set_nx(get_event_id_key(), 0).await?)
    }
}

impl Store for RedisRepo {
    async fn create_event(&self, mut event: Event) -> Result<(), Error> {
        let mut con = self.pool.get().await?;

        let event_field = con
            .incr(get_event_id_key(), 1)
            .await
            .map(|id| get_event_field(event.id.insert(id), event.ty))?;

        con.hset(get_event_key(), event_field, event.clone())
            .await?;

        con.publish(EVENT_CHANNEL_NAME, event).await?;

        Ok(())
    }

    async fn get_events(&self, user_id: &Id) -> Result<Vec<Event>, Error> {
        let mut con = self.pool.get().await?;

        let mut events = vec![];
        let maybe_event_ty: Option<EventType> = con.get(get_user_key(user_id)).await?;
        let event_ty = maybe_event_ty.ok_or(Error::NotFound("User"))?;
        let pattern = get_event_filed_pattern(event_ty);
        let mut iter: AsyncIter<(String, Event)> =
            con.hscan_match(get_event_key(), pattern).await?;
        while let Some(event) = iter.next_item().await {
            events.push(event.1);
        }

        Ok(events)
    }

    async fn create_subscription(&self, sub: Subscription) -> Result<(), Error> {
        let mut con = self.pool.get().await?;

        let key = get_user_key(&sub.user_id);
        if let Some(event_ty) = con.getset(&key, sub.event_ty).await? {
            con.set(&key, sub.event_ty | event_ty).await?;
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct RedisEventListener {
    tx: broadcast::Sender<Event>,
}

impl RedisEventListener {
    pub async fn new(params: impl AsRef<str>, channel: impl AsRef<str>) -> Result<Self, Error> {
        let (tx, mut rx) = broadcast::channel(100);
        let client = redis::Client::open(params.as_ref())?;
        let mut pubsub = client.get_async_pubsub().await?;
        pubsub.subscribe(channel.as_ref()).await?;

        tokio::spawn({
            let mut stream = pubsub.into_on_message();
            let tx = tx.clone();
            async move {
                loop {
                    select! {
                        msg = rx.recv() => debug!(?msg),
                        Some(msg) = stream.next() => {
                            let event: Event = msg.get_payload().unwrap();
                            tx.send(event).unwrap();
                        }
                    }
                }
            }
        });

        Ok(Self { tx })
    }
}

impl ListenEvents for RedisEventListener {
    fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }
}
