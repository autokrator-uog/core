/// This module contains schema parts which are common between incoming and outgoing messages.

pub type SequenceKey = String;
pub type SequenceValue = u32;

#[derive(Serialize, Deserialize, Clone)]
pub struct Consistency {
    pub key: SequenceKey,
    pub value: SequenceValue,
}
