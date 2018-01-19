use std::net::SocketAddr;

use actix::{Actor, Address, Context, Handler, Response, ResponseType};
use chrono::Local;
use failure::{Error, ResultExt};
use serde::Serialize;
use serde_json::{from_str, to_string, to_string_pretty, Value};
use sha1::Sha1;

use bus::Bus;
use error::ErrorKind;
use messages::SendToClient;
use schemas;
use session::Session;

/// The `Register` message is sent to the Bus when a client wants to provide more information about
/// itself or limit event types it can receive.
pub struct Register {
    pub message: String,
    pub sender: Address<Session>,
    pub bus: Address<Bus>,
}

impl ResponseType for Register {
    type Item = ();
    type Error = ();
}

impl Bus {
    fn register_send_to_kafka<T: Serialize>(&mut self, event: &T, event_type: &String) -> Result<(), Error> {
        let serialized = to_string(event).context(
            ErrorKind::SerializeJsonForSending)?;
        let pretty_serialized = to_string_pretty(event).context(
            ErrorKind::SerializeJsonForSending)?;

        info!("sending event to kafka: key='{}' topic='{}' event=\n{}",
              event_type, self.topic, pretty_serialized);
        self.producer.send_copy::<String, String>(&self.topic, None, Some(&serialized),
                                                  Some(event_type), None, 1000);
        Ok(())
    }

    fn register_hash_json(&mut self, input: Value) -> Result<String, Error> {
        let json = to_string(&input).context(ErrorKind::SerializeJsonForHashing)?;
        let mut hasher = Sha1::new();
        hasher.update(json.as_bytes());
        Ok(hasher.digest().to_string())
    }

    pub fn register_message(&mut self, message: Register) -> Result<(), Error> {
        let addr = message.sender;

        let mut receipt = schemas::outgoing::ReceiptMessage {
            receipts: Vec::new(),
            message_type: "receipt".to_string(),
            timestamp: Local::now().to_rfc2822(),
            sender: format!("{:?}", addr.clone()),
        };

        let parsed: schemas::incoming::RegisterMessage = from_str(&message.message).context(
            ErrorKind::ParseNewEventMessage)?;
        info!("parsed new event message: message=\n{}",
              to_string_pretty(&parsed).context(ErrorKind::SerializeJsonForSending)?);

        for raw_event in parsed.event_types.iter() {
            let event = schemas::kafka::EventMessage {
                timestamp: receipt.timestamp.clone(),
                sender: receipt.sender.clone(),
                event_type: raw_event.event_type.clone(),
                data: raw_event.data.clone()
            };

            receipt.receipts.push(schemas::outgoing::Receipt {
                checksum: self.register_hash_json(event.data.clone())?,
                status: "success".to_string()
            });

            self.register_send_to_kafka(&event, &event.event_type)?;
        }

        info!("sending receipt to the client");
        session.send(SendToClient(receipt));

        Ok(())
    }
}


impl Handler<Register> for Bus {
    fn handle(&mut self, _message: Register, _: &mut Context<Self>) -> Response<Self, Register> {
        Self::empty()
    }
}
