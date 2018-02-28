use schemas::Event;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rebuild {
    pub message_type: String,
    pub events: Vec<Event>,
}
