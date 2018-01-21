
use std::{thread, time};

use futures::{Stream};
use failure::{Error, Fail};
use couchbase::{Bucket, Cluster, CouchbaseError, N1qlResult};

use error::ErrorKind;


const BUCKET_NAME : &str = "events";
const MAX_RETRIES : u8 = 10;
const RETRY_INTERVAL_MILLIS : u64 = 1000;


fn create_gsi(bucket: &Bucket, name: &str) -> Result<(), Error> {
    let query = format!("CREATE INDEX {0} ON events ({0}) USING GSI", name);
    
    let event_type_index_result = bucket.query_n1ql(query).wait();
    for row in event_type_index_result {
        match row {
            Ok(N1qlResult::Meta(meta)) => {
                if meta.status() == "success" {
                    info!("Creating index {}... success!", name);
                }
                else {
                    warn!("Status of operation not 'success'... Failed to create GSI {}.", name);
                    return Err(Error::from(ErrorKind::CouchbaseCreateGSIFailed))
                }
            },
            Ok(N1qlResult::Row(row)) => panic!("shoudn't return rows... Returned: {:?}", row),
            Err(err) => {
                warn!("Failed to create GSI {}. Error: {}", name, err);
                return Err(Error::from(ErrorKind::CouchbaseCreateGSIFailed))
            }
        }
    }
    
    Ok(())
}


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
                
                
            }
            Err(err) => {
                error!("Failed to connect to couchbase - bucket {} on {}... [retries left: {}].  Error: {}", BUCKET_NAME, couchbase_host, retries, err);
                
                bucket = cluster.open_bucket(BUCKET_NAME, None);
            }
        }
        
        retries -= 1;
        thread::sleep(time::Duration::from_millis(RETRY_INTERVAL_MILLIS));
    }
    
    let bucket = match bucket {
        Ok(b) => b,
        Err(e) => {
            return Err(Error::from(e.context(ErrorKind::CouchbaseFailedConnect))) 
        }
    };
    
    // create Global Secondary Index for event_type field - required for querying on it
    let event_type_gsi_result = create_gsi(&bucket, "event_type");
    if event_type_gsi_result.is_err() {
        info!("'event_type' GSI creation failed, but proceeding as it could already exist...");
    }
    
    let timestamp_raw_gsi_result = create_gsi(&bucket, "timestamp_raw");
    if timestamp_raw_gsi_result.is_err() {
        info!("'timestamp_raw' GSI creation failed, but proceeding as it could already exist...");
    }
    
    Ok(bucket)
}
