use std::net::SocketAddr;

use actix::{Address, Context, Handler, ResponseType};
use chrono::Local;
use couchbase::{Document, BinaryDocument};
use failure::{Error, ResultExt};
use futures::Future;
use serde::Serialize;
use serde_json::{from_str, to_string, to_string_pretty};
use sha1::Sha1;

use bus::Bus;
use error::ErrorKind;
use schemas;
use session::Session;
use signals::SendToClient;

/// The `NewEvent` message is sent to the Bus when new events are sent from websockets.
pub struct NewEvent {
    pub message: String,
    pub sender: (Address<Session>, SocketAddr),
    pub bus: Address<Bus>,
    pub session_id: usize,
}

impl ResponseType for NewEvent {
    type Item = ();
    type Error = ();
}

impl Bus {
    fn send_to_kafka<T: Serialize>(&mut self, event: &T, event_type: &String) -> Result<(), Error> {
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

    fn persist_to_couchbase<T: Serialize>(&mut self, event: &T, document_id: &str) -> Result<(), Error> {
        let serialized = to_string(event).context(
            ErrorKind::SerializeJsonForSending)?;
        let pretty_serialized = to_string_pretty(event).context(
            ErrorKind::SerializeJsonForSending)?;

        let document = BinaryDocument::create(document_id, None, Some(serialized.as_bytes().to_owned()), None);

        info!("saving event in couchbase: event=\n{}", pretty_serialized);
        self.couchbase_bucket.upsert(document).wait().expect("Upsert failed!");

        Ok(())
    }

    fn hash<T: Serialize>(&mut self, input: &T) -> Result<String, Error> {
        let json = to_string(input).context(ErrorKind::SerializeJsonForHashing)?;

        let mut hasher = Sha1::new();
        hasher.update(json.as_bytes());
        Ok(hasher.digest().to_string())
    }

    pub fn process_new_event(&mut self, message: NewEvent) -> Result<(), Error> {
        let (session, addr) = message.sender;

        let now_time = Local::now();

        let mut receipt = schemas::outgoing::ReceiptMessage {
            receipts: Vec::new(),
            message_type: "receipt".to_string(),
            timestamp: now_time.to_rfc2822(),
            sender: format!("{:?}", addr.clone()),
        };

        let parsed: schemas::incoming::NewEventMessage = from_str(&message.message).context(
            ErrorKind::ParseNewEventMessage)?;
        info!("parsed new event message: message=\n{}",
              to_string_pretty(&parsed).context(ErrorKind::SerializeJsonForSending)?);

        for raw_event in parsed.events.iter() {
            let event = schemas::kafka::EventMessage {
                timestamp: now_time.to_rfc2822(),
                timestamp_raw: now_time.timestamp(), // store the timestamp in raw form too - easier to query
                sender: receipt.sender.clone(),
                event_type: raw_event.event_type.clone(),
                data: raw_event.data.clone(),
                correlation_id: raw_event.correlation_id,
                session_id: message.session_id,
            };

            receipt.receipts.push(schemas::outgoing::Receipt {
                // here, we just care about verifying the integrity of the data, so the hash need only be done on this
                checksum: self.hash(&event.data.clone())?,
                status: "success".to_string()
            });
            self.send_to_kafka(&event, &event.event_type)?;

            // do a separate hash to include timestamp, sender etc to make the hash always unique
            let hash_all = self.hash(&event.clone())?;
            self.persist_to_couchbase(&event, &hash_all.to_string())?;
        }

        info!("sending receipt to the client");
        session.send(SendToClient(receipt));

        Ok(())
    }
}

impl Handler<NewEvent> for Bus {
    type Result = ();

    fn handle(&mut self, message: NewEvent, _: &mut Context<Self>) {
        if let Err(e) = self.process_new_event(message) {
            error!("processing new event: error='{}'", e);
        }
    }
}
