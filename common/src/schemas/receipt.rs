#[derive(Serialize, Deserialize, Clone)]
pub struct Receipts {
    pub message_type: String,
    pub receipts: Vec<Receipt>,
    pub sender: String,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Hash, Clone)]
pub struct Receipt {
    pub checksum: String,
    pub status: String,
}
