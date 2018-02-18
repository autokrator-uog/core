use std::net::SocketAddr;

use actix::{Address, Context, Handler, ResponseType};
use chrono::Local;
use common::hash_json;
use common::schemas::{
    Consistency,
    ConsistencyValue,
    Event,
    NewEvents,
    Receipt,
    Receipts,
};
use couchbase::{Document, BinaryDocument};
use failure::{Error, ResultExt};
use futures::Future;
use serde::Serialize;
use serde_json::{from_str, to_string, to_string_pretty};

use bus::Bus;
use error::ErrorKind;
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
    fn send_to_kafka<T: Serialize>(&mut self, event: &T,
                                   event_type: &String) -> Result<(), Error> {
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

    fn persist_to_couchbase<T: Serialize>(&mut self, event: &T,
                                          document_id: &str) -> Result<(), Error> {
        let serialized = to_string(event).context(
            ErrorKind::SerializeJsonForSending)?;
        let pretty_serialized = to_string_pretty(event).context(
            ErrorKind::SerializeJsonForSending)?;

        let document = BinaryDocument::create(document_id, None,
                                              Some(serialized.as_bytes().to_owned()), None);

        info!("saving event in couchbase: event=\n{}", pretty_serialized);
        self.event_bucket.upsert(document).wait()?;

        Ok(())
    }

    fn persist_consistency_to_couchbase(&mut self) -> Result<(), Error> {
        let map = self.consistency.clone();
        let serialized = to_string(&map).context(
            ErrorKind::SerializeHashMapForCouchbase)?;

        let document = BinaryDocument::create("consistency", None,
                                              Some(serialized.as_bytes().to_owned()), None);

        info!("persisting updated consistency values to couchbase");
        self.consistency_bucket.upsert(document).wait()?;

        Ok(())
    }

    pub fn process_new_event(&mut self, message: NewEvent) -> Result<(), Error> {
        let (session, addr) = message.sender;

        let now_time = Local::now();

        let mut receipt = Receipts {
            receipts: Vec::new(),
            message_type: "receipt".to_string(),
            timestamp: now_time.to_rfc2822(),
            sender: format!("{:?}", addr.clone()),
        };

        let parsed: NewEvents = from_str(&message.message).context(
            ErrorKind::ParseNewEventMessage)?;
        info!("parsed new event message: message=\n{}",
              to_string_pretty(&parsed).context(ErrorKind::SerializeJsonForSending)?);

        for raw_event in parsed.events.iter() {
            let key = raw_event.consistency.key.clone();

            let value = match raw_event.consistency.value.clone() {
                ConsistencyValue::Explicit(v) => v,
                ConsistencyValue::Implicit => {
                    if let Some(&ConsistencyValue::Explicit(x)) = self.consistency.get(&key) {
                        x + 1
                    } else {
                        0
                    }
                },
            };
            let value = ConsistencyValue::Explicit(value);

            let consistency = Consistency { key: key, value: value.clone() };
            let event = Event {
                consistency: consistency,
                correlation_id: raw_event.correlation_id,
                data: raw_event.data.clone(),
                event_type: raw_event.event_type.clone(),
                message_type: None,
                timestamp: now_time.to_rfc2822(),
                // Store the timestamp in raw form too - easier to query.
                timestamp_raw: Some(now_time.timestamp()),
                sender: receipt.sender.clone(),
                session_id: Some(message.session_id),
            };

            let key = raw_event.consistency.key.clone();
            let success = if let Some(&ConsistencyValue::Explicit(x)) = self.consistency.get(&key) {
                value == ConsistencyValue::Explicit(x + 1)
            } else {
                value == ConsistencyValue::Explicit(0)
            };

            let mut status = "inconsistent";
            if success {
                self.consistency.insert(key, value.clone());
                if let Err(e) = self.persist_consistency_to_couchbase() {
                    warn!("failed to save consistency to couchbase: error='{:?}'", e);
                } 
                status = "success";

                info!("sending event to kafka: sequence_key='{}', sequence_value='{}'",
                      raw_event.consistency.key.clone(), value);
                self.send_to_kafka(&event, &event.event_type)?;

                // Do a separate hash to include timestamp, sender etc to make the hash always
                // unique.
                let hash_all = hash_json(&event.clone())?;
                info!("sending event to couchbase");
                self.persist_to_couchbase(&event, &hash_all.to_string())?;
            }

            receipt.receipts.push(Receipt {
                // We just care about verifying the integrity of the data,
                // so the hash need only be done on this.
                checksum: hash_json(&event.data.clone())?,
                status: status.to_string()
            });
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
