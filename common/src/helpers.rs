use failure::{Error, ResultExt};
use serde::Serialize;
use serde_json::to_string;
use sha1::Sha1;

use error::ErrorKind;

pub fn hash_json<T: Serialize>(input: &T) -> Result<String, Error> {
    let json = to_string(input).context(ErrorKind::SerializeJsonForHashing)?;

    let mut hasher = Sha1::new();
    hasher.update(json.as_bytes());
    Ok(hasher.digest().to_string())
}
