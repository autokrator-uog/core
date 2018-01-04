use common::{Event, EventContents};
use err::{ErrorKind, Result};
use producer::MessageContents;
use state::EventLoopState;

use chrono::Local;
use failure::ResultExt;
use serde_json::{to_string, Value};

use sha1::Sha1;

#[derive(Serialize, Deserialize)]
pub struct NewEvent {
    pub event_type: String,
    pub data: EventContents,
}

#[derive(Serialize, Deserialize)]
pub struct NewEventMessage {
    pub events: Vec<NewEvent>,
}

#[derive(Serialize, Deserialize, Hash)]
pub struct Receipt {
    pub checksum: String,
    pub status: String

}

#[derive(Serialize, Deserialize)]
pub struct ReceiptsMessage {
    pub receipts: Vec<Receipt>,
    pub message_type: String,
    pub timestamp: String,
    pub sender: String
}

impl MessageContents for NewEventMessage {
      fn process(&self, addr: String, state: &EventLoopState) -> Result<()> {
        let mut receipts_message = ReceiptsMessage {
            receipts: Vec::new(),
            message_type: "receipt".to_string(),
            timestamp: Local::now().to_rfc2822(),
            sender: addr.clone()
        };

        for raw_event in self.events.iter() {
            let producer = &state.producer;
            let topic = &state.topic;

            let event = Event {
                message_type: "new".to_string(),
                timestamp: receipts_message.timestamp.clone(),
                addr: addr.clone(),
                event_type: raw_event.event_type.clone(),
                data: raw_event.data.clone()
            };

            receipts_message.receipts.push(Receipt{
                checksum: data_hash(event.data.clone())?,
                status: "success".to_string()
            });

            info!("Sending {:?} to key {:?} on topic {:?}", event, event.event_type, topic);
            let serialized_event = to_string(&event).context(ErrorKind::JsonSerializationForHash)?;
            producer.send_copy::<String, String>(&topic, None,
                                                 Some(&serialized_event), Some(&event.event_type),
                                                 None, 1000);
        }

        info!("Sending receipt to the consumer");
        let serialized_receipt = to_string(&receipts_message).context(
            ErrorKind::JsonSerializationForReceipt)?;
        state.send_channel_out.unbounded_send((addr.clone(), serialized_receipt)).context(
            ErrorKind::ReceiptOnSendChannel)?;
        Ok(())
    }
}

fn data_hash(input: Value) -> Result<String> {
    let json = to_string(&input).context(ErrorKind::JsonSerializationForHash)?;
    let mut hasher = Sha1::new();
    hasher.update(json.as_bytes());
    Ok(hasher.digest().to_string())
}
