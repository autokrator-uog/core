use std::fmt;
use std::str::FromStr;
use std::marker::PhantomData;

use failure::{ResultExt, Error};
use serde::de::{self, Deserializer, Visitor};
use serde::ser::{Serialize, Serializer};

use error::ErrorKind;

pub type ConsistencyKey = String;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Ord, Serialize)]
pub enum ConsistencyValue {
    Implicit,
    Explicit(u32),
}

impl ConsistencyValue {
    pub fn value(&self) -> Result<u32, Error> {
        match *self {
            ConsistencyValue::Implicit => Err(Error::from(
                    ErrorKind::GetValueOfImplicitConsistency)),
            ConsistencyValue::Explicit(v) => Ok(v),
        }
    }
}

impl Default for ConsistencyValue {
    fn default() -> Self { ConsistencyValue::Implicit }
}

impl fmt::Display for ConsistencyValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ConsistencyValue::Implicit => write!(f, "implicit"),
            ConsistencyValue::Explicit(v) => v.fmt(f),
        }
    }
}

impl FromStr for ConsistencyValue {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "*" {
            Ok(ConsistencyValue::Implicit)
        } else {
            let parsed: u32 = s.parse::<u32>().context(ErrorKind::ParseConsistencyValue)?;
            Ok(ConsistencyValue::Explicit(parsed))
        }
    }
}

fn serialize_consistency_value<S>(value: &ConsistencyValue,
                                  serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer
{
    match *value {
        ConsistencyValue::Explicit(x) => x.serialize(serializer),
        ConsistencyValue::Implicit => "*".serialize(serializer),
    }
}

fn deserialize_consistency_value<'de, D>(deserializer: D) -> Result<ConsistencyValue, D::Error>
    where D: Deserializer<'de>
{
    struct ConsistencyValueParser(PhantomData<fn() -> ConsistencyValue>);

    impl<'de> Visitor<'de> for ConsistencyValueParser
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

        fn visit_str<E: de::Error>(self, value: &str) -> Result<ConsistencyValue, E> {
            FromStr::from_str(value).map_err(de::Error::custom)
        }
    }

    deserializer.deserialize_any(ConsistencyValueParser(PhantomData))
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Consistency {
    pub key: ConsistencyKey,

    /// Incoming consistency values can be of two valid types - implicit or explicit.
    ///
    /// Implicit consistency values are represented by the string `"*"` and indicate that the
    /// event bus should take the next available value.
    ///
    /// Explicit consistency values are represented by numbers (either literal or as strings).
    #[serde(default, serialize_with="serialize_consistency_value",
            deserialize_with = "deserialize_consistency_value")]
    pub value: ConsistencyValue,
}

mod tests {
    #[test]
    fn parse_consistency_from_str() {
        assert_eq!(ConsistencyValue::from_str("*").unwrap(), ConsistencyValue::Implicit);
        assert_eq!(ConsistencyValue::from_str("1").unwrap(), ConsistencyValue::Explicit(1));
        assert_eq!(ConsistencyValue::from_str("1234").unwrap(), ConsistencyValue::Explicit(1234));
        assert!(ConsistencyValue::from_str("non_wildcard_or_number_string").is_err());
    }
}
