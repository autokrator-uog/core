// This module contains schema parts which are common between incoming and outgoing messages.
use bus::{SequenceKey, SequenceValue};

pub type Consistency = ConsistencyStruct;

#[derive(Serialize, Deserialize, Clone)]
pub struct ConsistencyStruct {
    pub key: SequenceKey,
    pub value: SequenceValue,
}