pub mod kafka;

use crate::model::BusMessage;
use tokio::sync::mpsc::UnboundedReceiver;

pub trait NotifyBus {
    fn redirect(self, msg_rx: UnboundedReceiver<BusMessage>);
}
