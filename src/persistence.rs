use std::{thread, time};

use failure::{Error, Fail};
use futures::{Stream};
use couchbase::{Bucket, Cluster, CouchbaseError, N1qlResult};

use error::ErrorKind;

const BUCKET_NAME: &str = "events";
const MAX_RETRIES: u8 = 60;
const RETRY_INTERVAL_MILLIS: u64 = 1000;

fn create_gsi(bucket: &Bucket, name: &str) -> Result<(), Error> {
    let query = format!("CREATE INDEX {0} ON {1} ({0}) USING GSI", name, BUCKET_NAME);

    let event_type_index_result = bucket.query_n1ql(query).wait();
    for row in event_type_index_result {
        match row {
            Err(err) => {
                warn!("failed to create global secondary index: gsi='{}' error='{}'", name, err);
                return Err(Error::from(ErrorKind::CouchbaseCreateGSIFailed))
            }
            Ok(N1qlResult::Row(_)) => return Err(Error::from(
                    ErrorKind::CouchbaseUnexpectedResultReturned)),
            Ok(N1qlResult::Meta(meta)) => {
                if meta.status() == "success" {
                    info!("creating global secondary index successful: gsi='{}'", name);
                } else {
                    warn!("creating global secondary index unsuccessful: gsi='{}'", name);
                    return Err(Error::from(ErrorKind::CouchbaseCreateGSIFailed))
                }
            },
        }
    }

    Ok(())
}

pub fn connect_to_bucket(couchbase_host: &str) -> Result<Bucket, Error> {
    // This is simply a state object, it doesn't actually initiate connections.
    let mut cluster = Cluster::new(couchbase_host)?;
    cluster.authenticate("connect", "connect");
    let cluster = cluster;

    // Retry logic on opening bucket.
    let mut retries = MAX_RETRIES;
    let mut bucket = cluster.open_bucket(BUCKET_NAME, None);

    while bucket.is_err() && retries > 0 {
        match bucket {
            Ok(_) => {
                return Err(Error::from(ErrorKind::CouchbaseUnexpectedResultReturned))
            },
            // This is the error that is called if the bucket does not exist, somehow...
            Err(CouchbaseError::AuthFailed) => {
                warn!("the bucket does not exist. waiting for it to be created: \
                      bucket='{}' retries_remaining='{}'", BUCKET_NAME, retries);
            },
            Err(err) => {
                error!("failed to connect to couchbase: bucket='{}' host='{}' \
                       error='{}' retries_remaining='{}'",
                       BUCKET_NAME, couchbase_host, err, retries);
            },
        }

        retries -= 1;
        bucket = cluster.open_bucket(BUCKET_NAME, None);
        thread::sleep(time::Duration::from_millis(RETRY_INTERVAL_MILLIS));
    }

    let bucket = match bucket {
        Ok(bucket) => {
            info!("successfully connected to couchbase bucket: bucket='{}'", BUCKET_NAME);
            bucket
        },
        Err(e) => {
            return Err(Error::from(e.context(ErrorKind::CouchbaseFailedConnect)))
        }
    };

    // Create Global Secondary Index for event_type field - required for querying on it.
    let event_type_gsi_result = create_gsi(&bucket, "event_type");
    if event_type_gsi_result.is_err() {
        info!("global secondary index creation failed, may already exist: gsi='event_type'");
    }

    let timestamp_raw_gsi_result = create_gsi(&bucket, "timestamp_raw");
    if timestamp_raw_gsi_result.is_err() {
        info!("global secondary index creation failed, may already exist: gsi='timestamp_raw'");
    }

    Ok(bucket)
}
