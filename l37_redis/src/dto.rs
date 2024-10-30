use crate::model::{Event, EventType};
use anyhow::anyhow;
use chrono::Utc;
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
pub struct EventDto {
    #[serde(rename(deserialize = "type"))]
    #[serde(deserialize_with = "de_event_type")]
    pub ty: EventType,
    pub data: Option<String>,
}

fn de_event_type<'de, D>(deserializer: D) -> Result<EventType, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    EventType::deserialize(deserializer).and_then(|ty| {
        if ty.bits().count_ones() == 1 {
            return Ok(ty);
        }

        let variants = EventType::all()
            .iter_names()
            .map(|(name, _)| name)
            .collect::<Vec<_>>()
            .join(" | ");

        Err(Error::custom(anyhow!("expected variants are {}", variants)))
    })
}

impl From<EventDto> for Event {
    fn from(value: EventDto) -> Self {
        Self {
            id: None,
            ty: value.ty,
            ts: Utc::now(),
            data: value.data,
        }
    }
}
