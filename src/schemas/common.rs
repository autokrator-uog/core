// This module contains schema parts which are common between incoming and outgoing messages.
use bus::{SequenceKey, SequenceValue};

#[derive(Serialize, Deserialize, Clone)]
pub struct Consistency {
    pub key: SequenceKey,
    pub value: SequenceValue,
}