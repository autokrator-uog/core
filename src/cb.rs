
use std::{thread, time};

use failure::{Error, Fail};
use couchbase::{Bucket, Cluster, CouchbaseError};

use error::ErrorKind;


const BUCKET_NAME : &str = "events";
const MAX_RETRIES : u8 = 10;
const RETRY_INTERVAL_MILLIS : u64 = 1000;


pub fn cb_connect_to_bucket(couchbase_host: &str) -> Result<Bucket, Error> {
    // this is simply a state object, it doesn't actually initiate connections
    let mut cluster = Cluster::new(couchbase_host)?;
    cluster.authenticate("connect", "connect");
    let cluster = cluster;
    
    // retry logic on opening bucket
    let mut retries = MAX_RETRIES;
    let mut bucket = cluster.open_bucket(BUCKET_NAME, None);
    
    while bucket.is_err() && retries > 0 {
        match bucket {
            Ok(_) => {
                panic!("bucket.is_err() shoud mean this never happens!");
            }
            Err(CouchbaseError::AuthFailed) => { // this is the error that is called if the bucket does not exist, somehow...
                warn!("The 'events' bucket does not exist. Attempting to create it... [retries left: {}]", retries);
                
                // TODO try to create the events bucket.
                
                // CREATE INDEX event_type ON events (event_type) USING GSI
            }
            Err(err) => {
                error!("Failed to connect to couchbase - bucket {} on {}... [retries left: {}].  Error: {}", BUCKET_NAME, couchbase_host, retries, err);
                
                bucket = cluster.open_bucket(BUCKET_NAME, None);
            }
        }
        
        retries -= 1;
        thread::sleep(time::Duration::from_millis(RETRY_INTERVAL_MILLIS));
    }
    
    match bucket {
        Ok(b) => Ok(b),
        Err(e) => {
            return Err(Error::from(e.context(ErrorKind::CouchbaseFailedConnect))) 
        }
    }
}
