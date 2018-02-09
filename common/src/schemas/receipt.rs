#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipts {
    pub message_type: String,
    pub receipts: Vec<Receipt>,
    pub sender: String,
    pub timestamp: String,
}

#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct Receipt {
    pub checksum: String,
    pub status: String,
}
