use actix::{Address, Context, Handler, ResponseType};
use chrono::DateTime;
use common::schemas::{Event, Rebuild, Query as QuerySchema};
use couchbase::{N1qlResult};
use failure::{Error, Fail, ResultExt};
use futures::{Stream};
use serde_json::{from_str, to_string_pretty};

use bus::Bus;
use error::ErrorKind;
use session::Session;
use signals::SendToClient;

/// The `Query` message is sent to the Bus when query requests are sent from websockets.
pub struct Query {
    pub message: String,
    pub sender: Address<Session>,
    pub bus: Address<Bus>,
}

impl ResponseType for Query {
    type Item = ();
    type Error = ();
}

#[derive(Deserialize)]
pub struct CouchbaseStoredEvent {
    pub events: Event
}

impl Bus {
    pub fn process_query_message(&mut self, message: Query) -> Result<(), Error> {
        // parse the JSON message
        let parsed: QuerySchema = from_str(&message.message).context(
            ErrorKind::ParseQueryMessage)?;

        debug!("parsed query event message: message=\n{}",
              to_string_pretty(&parsed).context(ErrorKind::SerializeJsonForSending)?);

        let query_timestamp = if parsed.since.to_uppercase() == "*" {
            0
        } else {
            let begin_datetime = DateTime::parse_from_rfc3339(&parsed.since).context(
                ErrorKind::ParseQueryMessage)?;
            begin_datetime.timestamp()
        };

        // serialize event types into a sensible string
        let event_types: Vec<String> = parsed.event_types.iter()
            .map(|et| { "\"".to_string() + &et + "\"" })
            .collect();
        let query = format!(r#"
                                SELECT * FROM events
                                WHERE event_type IN [{}] AND timestamp_raw > {}
                                ORDER BY timestamp_raw ASC
                            "#,
                            event_types.join(", "), query_timestamp);
        debug!("executing query: query=\n{}", query);

        let client_session = message.sender;
        let result_iter = self.event_bucket.query_n1ql(query).wait();

        let mut rebuild = Rebuild {
            message_type: String::from("rebuild"),
            events: Vec::new(),
        };
        for row in result_iter {
            match row {
                Ok(N1qlResult::Meta(meta)) => {
                    // we don't really care about this, just spit it out for debug
                    debug!("raw meta received: meta='{:?}'", meta)
                },
                Ok(N1qlResult::Row(row)) => {
                    debug!("raw row received: row='{}'", &row.as_ref());

                    let parsed_row: CouchbaseStoredEvent = from_str(&row.as_ref()).context(
                        ErrorKind::CouchbaseDeserialize)?;
                    let mut event = parsed_row.events.clone();
                    event.message_type = Some(String::from("rebuild"));
                    rebuild.events.push(event);
                },
                Err(e) => return Err(Error::from(e.context(
                            ErrorKind::CouchbaseFailedGetQueryResult))),
            }
        }

        client_session.send(SendToClient(rebuild));
        Ok(())
    }
}

impl Handler<Query> for Bus {
    type Result = ();

    fn handle(&mut self, message: Query, _: &mut Context<Self>) {
        if let Err(e) = self.process_query_message(message) {
            error!("processing query message: error='{}'", e);
        }
    }
}
