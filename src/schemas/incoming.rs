/// This module contains the schemas for all the incoming messages we get. We don't include
///
/// the `message_type` field in these. Top-level structs are appended with 'Message'.
use serde_json::Value;
use schemas::common::Consistency;
use std::fmt;
use std::str::FromStr;
use std::marker::PhantomData;
use serde::de::{self, Deserialize, Deserializer, Visitor, MapAccess};
use serde_json::from_str;
use void::Void;
use bus::SequenceKey;

#[derive(Serialize, Deserialize, Debug)]
enum ConsistencyValue {
    Implicit,
    Explicit(u32),
}

impl Default for ConsistencyValue {
    fn default() -> Self { ConsistencyValue::Implicit }
}

#[derive(Serialize, Deserialize, Debug)]
struct ConsistencyIn {
    key: SequenceKey,

    // this lets it handle the case where there is no val field provided.
    #[serde(default, deserialize_with = "consistency_value_parse")]
    val: ConsistencyValue,
}

impl FromStr for ConsistencyValue {
    // This implementation of `from_str` can never fail, so use the impossible
    // `Void` type as the error type.
    type Err = Void;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "*" {
            Ok(ConsistencyValue::Implicit)
        } else if s == "a number" {
            // parse it
            Ok(ConsistencyValue::Explicit(12))
        } else {
            // return with our actual error type
            Ok(ConsistencyValue::Explicit(12))
        }
    }
}

fn consistency_value_parse<'de, D>(deserializer: D) -> Result<ConsistencyValue, D::Error>
    where D: Deserializer<'de>
{
    // This is a Visitor that forwards string types to T's `FromStr` impl and
    // forwards map types to T's `Deserialize` impl. The `PhantomData` is to
    // keep the compiler from complaining about T being an unused generic type
    // parameter. We need T in order to know the Value type for the Visitor
    // impl.
    struct ConstValParse(PhantomData<fn() -> ConsistencyValue>);

    impl<'de> Visitor<'de> for ConstValParse
    {
        type Value = ConsistencyValue;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or integer")
        }
        
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<ConsistencyValue, E> {
            Ok(ConsistencyValue::Explicit(v as u32))
        }
        
        fn visit_u32<E: de::Error>(self, v: u32) -> Result<ConsistencyValue, E> {
            Ok(ConsistencyValue::Explicit(v))
        }
        
        fn visit_u16<E: de::Error>(self, v: u16) -> Result<ConsistencyValue, E> {
            Ok(ConsistencyValue::Explicit(v as u32))
        }
        
        fn visit_u8<E: de::Error>(self, v: u8) -> Result<ConsistencyValue, E> {
            Ok(ConsistencyValue::Explicit(v as u32))
        }

        fn visit_str<E>(self, value: &str) -> Result<ConsistencyValue, E>
            where E: de::Error
        {
            Ok(FromStr::from_str(value).unwrap())
        }
    }

    deserializer.deserialize_any(ConstValParse(PhantomData))
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NewEventMessage {
    pub events: Vec<Event>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Event {
    pub event_type: String,
    pub correlation_id: usize,
    pub data: Value,
    pub consistency: Consistency,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct QueryMessage {
    pub event_types: Vec<String>,
    pub since: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RegisterMessage {
    pub event_types: Vec<String>,
}

#[cfg(test)]
mod event_tests {
    use super::*;
    use serde_json::from_str;

    #[test]
    fn parse_new_message_type() {
        let data = r#"{
                        "message_type": "new",
                        "events": [
                            {
                                "event_type": "deposit",
                                "correlation_id": 94859829321,
                                "data": {
                                    "account": 837,
                                    "amount": 3
                                },
                                "consistency": {
                                    "key": "testkey",
                                    "value": "*"
                                }
                            },
                            {
                                "event_type": "withdrawal",
                                "correlation_id": 94859829321,
                                "data": {
                                    "account": 2837,
                                    "amount": 5
                                },
                                "consistency": {
                                    "key": "testkey",
                                    "value": 123456
                                }
                            }
                        ]
                   }"#;
        let parsed: Result<NewEventMessage, _> = from_str(data);

        assert!(parsed.is_ok());
        if let Ok(message) = parsed {
            assert_eq!(message.events[0].event_type, "deposit");
            assert_eq!(message.events[0].correlation_id, 94859829321);
            assert_eq!(message.events[0].data["account"], 837);
            assert_eq!(message.events[0].data["amount"], 3);
            //assert_eq!(message.events[0].consistency.key, "testkey");
            //assert_eq!(message.events[0].consistency.value, 123456);
            assert_eq!(message.events[1].event_type, "withdrawal");
            assert_eq!(message.events[1].correlation_id, 94859829321);
            assert_eq!(message.events[1].data["account"], 2837);
            assert_eq!(message.events[1].data["amount"], 5);
            //assert_eq!(message.events[1].consistency.key, "testkey");
            //assert_eq!(message.events[1].consistency.value, 123456);
        }
    }

    #[test]
    fn parse_register_message_type() {
        let data = r#"{
                        "message_type": "register",
                        "event_types": [
                            "deposit",
                            "withdrawal"
                        ]
                   }"#;
        let parsed: Result<RegisterMessage, _> = from_str(data);

        assert!(parsed.is_ok());
        if let Ok(message) = parsed {
            assert_eq!(message.event_types[0], "deposit");
            assert_eq!(message.event_types[1], "withdrawal");
        }
    }

    #[test]
    fn parse_query_message_type() {
        let data = r#"{
                        "message_type": "query",
                        "event_types": [
                            "deposit",
                            "withdrawal"
                        ],
                        "since": "2010-06-09T15:20:00-07:00"
                   }"#;
        let parsed: Result<QueryMessage, _> = from_str(data);

        assert!(parsed.is_ok());
        if let Ok(message) = parsed {
            assert_eq!(message.event_types[0], "deposit");
            assert_eq!(message.event_types[1], "withdrawal");
            assert_eq!(message.since, "2010-06-09T15:20:00-07:00");
        }
    }
}
