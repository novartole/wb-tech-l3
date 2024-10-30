use crate::{
    model::{Event, EventType, Id, Subscription},
    repo::{ListenEvents, Store},
};
use dashmap::DashMap;
use std::sync::Arc;
use tracing::debug;

#[derive(Clone)]
pub struct AppState<R, L>
where
    R: Store,
    L: ListenEvents,
{
    pub repo: R,
    pub event_listener: L,
    pub sub_manager: SubManager,
}

#[derive(Clone, Default)]
pub struct SubManager {
    subs: Arc<DashMap<Id, EventType>>,
}

impl SubManager {
    pub fn register(&self, sub: Subscription) {
        self.subs.insert(sub.user_id, sub.event_ty);
        debug!(?self.subs, "registred new sub");
    }

    pub fn filter(&self, user_id: &Id, event: Event) -> Option<Event> {
        if self.subs.get(user_id)?.contains(event.ty) {
            Some(event)
        } else {
            None
        }
    }
}
