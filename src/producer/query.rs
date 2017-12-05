use rdkafka::client::EmptyContext;
use rdkafka::producer::FutureProducer;
use producer::MessageContents;

#[derive(Serialize, Deserialize)]
pub struct QueryMessage {
    pub event_types: Vec<String>,
    pub since: String,
}

impl MessageContents for QueryMessage {
    fn process(&self, _addr: String, _producer: FutureProducer<EmptyContext>, _topic: String) {
        unimplemented!();
    }
}
