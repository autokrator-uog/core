use actix::{Actor, Address, Context, Handler, Response, ResponseType};
use failure::{Error, Fail, ResultExt};
use serde_json::{from_str, to_string_pretty};
use couchbase::{N1qlResult};
use futures::{Stream};
use chrono::DateTime;

use schemas;
use schemas::kafka::EventMessage;
use signals::SendToClient;

use error::ErrorKind;
use bus::Bus;
use session::Session;

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
    pub events: EventMessage
}

impl Bus {
    pub fn process_query_message(&mut self, message: Query) -> Result<(), Error> {
        // parse the JSON message
        let parsed: schemas::incoming::QueryMessage = from_str(&message.message)
                .context(ErrorKind::ParseNewEventMessage)?;
        
        debug!("parsed query event message: message=\n{}",
              to_string_pretty(&parsed).context(ErrorKind::SerializeJsonForSending)?);
        
        // serialize event types into a sensible string
        let event_types: Vec<String> = parsed.event_types.iter()
                    .map(|et| { "\"".to_string() + &et + "\"" })
                    .collect();        
        let mut query = format!("SELECT * FROM events WHERE event_type IN [{}]", event_types.join(", "));
        
        // allow an ALL option
        if parsed.since.to_uppercase() != "*" {
            let begin_datetime = DateTime::parse_from_rfc3339(&parsed.since).context(ErrorKind::ParseQueryMessage)?;
            query = format!("{} AND timestamp_raw > {}", query, begin_datetime.timestamp());
        }
        
        debug!("executing query: query='{}'", query);
        
        let client_session = message.sender;
        let result_iter = self.couchbase_bucket.query_n1ql(query).wait();
        
        for row in result_iter {
            match row {
                Err(e) => return Err(Error::from(e.context(ErrorKind::CouchbaseFailedGetQueryResult))),
                Ok(N1qlResult::Meta(meta)) => {
                    // we don't really care about this, just spit it out for debug
                    debug!("raw meta received: meta='{:?}'", meta)
                }, 
                Ok(N1qlResult::Row(row)) => {
                    debug!("raw row received: row='{}'", &row.as_ref());
                    
                    let parsed_row: CouchbaseStoredEvent = from_str(&row.as_ref()).context(ErrorKind::CouchbaseDeserialize)?;
                    let event = parsed_row.events;
                    
                    client_session.send(SendToClient(event));
                },
            }
        }
        
        Ok(())
    }
}

impl Handler<Query> for Bus {
    fn handle(&mut self, message: Query, _: &mut Context<Self>) -> Response<Self, Query> {
        if let Err(e) = self.process_query_message(message) {
            error!("processing query message: error='{}'", e);
        }

        Self::empty()
    }
}
