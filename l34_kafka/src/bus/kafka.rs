use super::NotifyBus;
use crate::model::BusMessage;
use rdkafka::{
    error::KafkaResult,
    producer::{FutureProducer, FutureRecord},
    util::Timeout,
    ClientConfig,
};
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{error, trace};

pub struct KafkaMsgProducer {
    inner: FutureProducer,
}

impl KafkaMsgProducer {
    pub fn new(bootstrap_server: &str) -> KafkaResult<Self> {
        Ok(Self {
            inner: ClientConfig::new()
                .set("bootstrap.servers", bootstrap_server)
                .set("message.timeout.ms", "5000")
                .create()?,
        })
    }
}

impl NotifyBus for KafkaMsgProducer {
    fn redirect(self, mut msg_rx: UnboundedReceiver<BusMessage>) {
        tokio::spawn(async move {
            while let Some(msg) = msg_rx.recv().await {
                trace!("new message for bus");

                let (key, payload) = match msg {
                    BusMessage::User(val) => match serde_json::to_vec(&val) {
                        Err(why) => {
                            error!("failed serializing: {:?}", why);
                            break;
                        }
                        Ok(val) => ("users", val),
                    },
                    BusMessage::Product(val) => match serde_json::to_vec(&val) {
                        Err(why) => {
                            error!("failed serializing: {:?}", why);
                            break;
                        }
                        Ok(val) => ("products", val),
                    },
                };

                let record = FutureRecord::to("db").key(key).payload(&payload);

                if let Err((why, _)) = self.inner.send(record, Timeout::Never).await {
                    error!("failed sending notification: {:?}", why);
                    break;
                }
            }
        });
    }
}
