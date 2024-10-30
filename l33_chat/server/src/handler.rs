use crate::{
    error::Error,
    event::Event,
    model,
    notifier::Notifier,
    state::{AppState, StoreChat},
};
use anyhow::Context;
use axum::{
    extract::{
        ws::{self, close_code, CloseFrame, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use futures::{
    stream::{SplitSink, SplitStream, StreamExt},
    SinkExt,
};
use serde::Deserialize;
use std::borrow::Cow;
use tokio::{select, sync::broadcast::Receiver, task::JoinHandle};
use tracing::{error, trace, warn};
use validator::Validate;

pub async fn create_room<R>(
    State(state): State<AppState<R>>,
    Json(room): Json<model::Room>,
) -> Result<(), Error>
where
    R: StoreChat,
{
    state.repo.create_room(room).await?;
    Ok(())
}

pub async fn create_user<R>(
    State(state): State<AppState<R>>,
    Json(user): Json<model::User>,
) -> Result<(), Error>
where
    R: StoreChat,
{
    state.repo.create_user(user).await?;
    Ok(())
}

#[derive(Deserialize, Validate)]
pub struct Participant {
    #[validate(length(min = 7))]
    pub user_id: String,
    #[validate(length(min = 7))]
    pub room_id: String,
}

pub async fn join_room<R>(
    State(state): State<AppState<R>>,
    Json(
        ref payload @ Participant {
            ref user_id,
            ref room_id,
        },
    ): Json<Participant>,
) -> Result<StatusCode, Error>
where
    R: StoreChat,
{
    payload.validate()?;

    if !state.repo.add_user_to_room(user_id, room_id).await? {
        warn!(%user_id, %room_id, "user is already participant of room");
        return Ok(StatusCode::OK);
    }

    if let Some(event_tx) = state.pool.get(room_id) {
        let join_event = state.repo.get_user(user_id).await.map(Event::join)?;
        event_tx.send(join_event).context("Sending Join event")?;
    }

    Ok(StatusCode::CREATED)
}

pub async fn leave_room<R>(
    State(state): State<AppState<R>>,
    Json(
        ref payload @ Participant {
            ref user_id,
            ref room_id,
        },
    ): Json<Participant>,
) -> Result<StatusCode, Error>
where
    R: StoreChat,
{
    payload.validate()?;

    if !state.repo.is_user_in_room(user_id, room_id).await? {
        warn!(%user_id, %room_id, "user wasn't participant of room");
        return Ok(StatusCode::OK);
    }

    if let Some(event_tx) = state.pool.get(room_id) {
        let leave_event = state.repo.get_user(user_id).await.map(Event::leave)?;
        event_tx.send(leave_event).context("Sending Leave event")?;
    }

    if state.repo.remove_user_from_room(user_id, room_id).await? {
        warn!(%user_id, %room_id, "user wasn't participant of room");
        return Ok(StatusCode::OK);
    }

    Ok(StatusCode::CREATED)
}

pub async fn send_message<R>(
    State(state): State<AppState<R>>,
    Json(message): Json<model::Message>,
) -> Result<(), Error>
where
    R: StoreChat,
{
    message.validate()?;

    if !state
        .repo
        .is_user_in_room(&message.user_id, &message.room_id)
        .await?
    {
        return Err(Error::Forbidden);
    }

    let (user_id, room_id, text) = state.repo.create_message(message).await.map(
        |model::Message {
             user_id,
             room_id,
             text,
             ..
         }| (user_id, room_id, text),
    )?;

    if let Some(event_tx) = state.pool.get(&room_id) {
        let msg_event = state
            .repo
            .get_user(&user_id)
            .await
            .map(|user| Event::message(user, text))?;
        event_tx.send(msg_event).context("Sending Leave event")?;
    }

    Ok(())
}

pub async fn ws_messages<R>(
    ws: WebSocketUpgrade,
    State(state): State<AppState<R>>,
    Query(
        ref payload @ Participant {
            ref user_id,
            ref room_id,
        },
    ): Query<Participant>,
) -> Result<impl IntoResponse, Error>
where
    R: StoreChat,
{
    payload.validate()?;

    if !state.repo.is_user_in_room(user_id, room_id).await? {
        return Err(Error::Forbidden);
    }

    let user = state.repo.get_user(user_id).await?;

    let event_rx = state
        .pool
        .entry(room_id.to_string())
        .or_insert(Notifier::new())
        .subscribe();

    return Ok(ws.on_upgrade(move |socket| callback(socket, event_rx, user)));

    async fn callback(stream: WebSocket, event_rx: Receiver<Event>, user: model::User) {
        let (ws_tx, ws_rx) = stream.split();

        let mut send_task = spawn_sender(ws_tx, event_rx, user.clone());
        let mut recv_task = spawn_receiver(ws_rx, user);

        select! {
            _ = &mut send_task => recv_task.abort(),
            _ = &mut recv_task => send_task.abort(),
        };
    }

    fn spawn_sender(
        mut ws_tx: SplitSink<WebSocket, ws::Message>,
        mut event_rx: Receiver<Event>,
        user: model::User,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                break match event_rx.recv().await {
                    Ok(event) => {
                        if event.is_user_leaving(&user) {
                            let cf = {
                                let code = close_code::NORMAL;
                                let reason = Cow::from("User closed connection");
                                Some(CloseFrame { code, reason })
                            };
                            if let Err(why) = ws_tx.send(ws::Message::Close(cf)).await {
                                error!("failed sending close message: {:?}", why);
                            }
                        } else {
                            // match serde_json::to_vec(&event) {
                            match serde_json::to_string(&event) {
                                Ok(json) => {
                                    // if let Err(why) = sender.send(ws::Message::Binary(json)).await {
                                    if let Err(why) = ws_tx.send(ws::Message::Text(json)).await {
                                        error!("failed sending message: {:?}", why);
                                    } else {
                                        continue;
                                    }
                                }
                                Err(why) => error!("failed serializing message: {:?}", why),
                            }
                        }
                    }
                    Err(why) => error!("failed receiving event: {:?}", why),
                };
            }
        })
    }

    fn spawn_receiver(mut ws_rx: SplitStream<WebSocket>, user: model::User) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                if let Some(try_msg) = ws_rx.next().await {
                    match try_msg {
                        Ok(msg) => {
                            if let ws::Message::Close(maybe_cf) = msg {
                                break if let Some(cf) = maybe_cf {
                                    trace!(
                                        "{:?} sent close with code {} and reeason {}",
                                        user,
                                        cf.code,
                                        cf.reason
                                    );
                                } else {
                                    trace!("{:?} sent close", user);
                                };
                            }
                        }
                        Err(why) => error!("failed receiving message from {:?}: {:?}", why, user),
                    }
                }
            }
        })
    }
}
