#[derive(Serialize, Deserialize, Clone)]
pub struct Registration {
    pub client_type: String,
    pub event_types: Vec<String>,
    pub message_type: String,
}
