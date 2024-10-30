use crate::{
    dto::EventDto,
    error::Error,
    model::{self, Id, Subscription},
    repo::{ListenEvents, Store},
    state::AppState,
};
use axum::{
    extract::{Path, State},
    response::sse::{self, KeepAlive, Sse},
    Json,
};
use futures::{Stream, StreamExt};
use std::{convert::Infallible, time::Duration};

pub async fn create_event<R, L>(
    State(state): State<AppState<R, L>>,
    Json(event_dto): Json<EventDto>,
) -> Result<(), Error>
where
    R: Store,
    L: ListenEvents,
{
    state.repo.create_event(model::Event::from(event_dto)).await
}

pub async fn subscribe<R, L>(
    State(state): State<AppState<R, L>>,
    Json(sub): Json<Subscription>,
) -> Result<(), Error>
where
    R: Store,
    L: ListenEvents,
{
    state.repo.create_subscription(sub).await?;
    state.sub_manager.register(sub);
    Ok(())
}

pub async fn get_events<R, L>(
    State(state): State<AppState<R, L>>,
    Path(user_id): Path<Id>,
) -> Result<Sse<impl Stream<Item = Result<sse::Event, Infallible>>>, Error>
where
    R: Store,
    L: ListenEvents,
{
    // let history = {
    //     let events = state.repo.get_events(&user_id).await?;
    //     async_stream::stream! {
    //         for event in events {
    //              yield event;
    //         }
    //     }
    // };

    let recent = {
        let mut rx = state.event_listener.subscribe();
        async_stream::stream! {
            while let Ok(event) = rx.recv().await {
                if let Some(event) = state.sub_manager.filter(&user_id, event) {
                    yield event;
                }
            }
        }
    };

    // let stream = history
    //     .chain(recent)
    //     .map(|event| sse::Event::default().json_data(event).map(Ok).unwrap());

    let stream = recent.map(|event| sse::Event::default().json_data(event).map(Ok).unwrap());

    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new().interval(Duration::from_secs(2)), // default is 15 secs
    ))
}
