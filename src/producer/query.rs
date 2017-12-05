use err::Result;
use producer::MessageContents;
use state::EventLoopState;

#[derive(Serialize, Deserialize)]
pub struct QueryMessage {
    pub event_types: Vec<String>,
    pub since: String,
}

impl MessageContents for QueryMessage {
    fn process(&self, _addr: String, _state: &EventLoopState) -> Result<()> {
        unimplemented!();
    }
}
