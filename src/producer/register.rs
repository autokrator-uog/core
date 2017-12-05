use err::Result;
use producer::MessageContents;
use state::EventLoopState;

#[derive(Serialize, Deserialize)]
pub struct RegisterMessage {
    pub event_types: Vec<String>,
}

impl MessageContents for RegisterMessage {
    fn process(&self, _addr: String, _state: &EventLoopState) -> Result<()> {
        unimplemented!();
    }
}
