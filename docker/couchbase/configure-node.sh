#!/bin/bash

set -x
set -m

/entrypoint.sh couchbase-server &

sleep 15

# init node
curl -v -X POST http://127.0.0.1:8091/nodes/self/controller/settings \
  -d 'data_path=%2Fopt%2Fcouchbase%2Fvar%2Flib%2Fcouchbase%2Fdata& \
  index_path=%2Fopt%2Fcouchbase%2Fvar%2Flib%2Fcouchbase%2Fdata'

# rename to "couchbase.db"
curl -v -X POST http://127.0.0.1:8091/node/controller/rename -d 'hostname=couchbase.db'

# Setup index and memory quota
curl -v -X POST http://127.0.0.1:8091/pools/default -d memoryQuota=2048 -d indexMemoryQuota=256

# Setup services
curl -v http://127.0.0.1:8091/node/controller/setupServices -d services=kv%2Cn1ql%2Cindex

# Setup credentials
curl -v http://127.0.0.1:8091/settings/web -d port=8091 -d username=connect -d password=connect

# Setup Memory Optimized Indexes
curl -i -u connect:connect -X POST http://127.0.0.1:8091/settings/indexes -d 'storageMode=memory_optimized'

# Create events buckeet
curl -v -u connect:connect -X POST http://127.0.0.1:8091/pools/default/buckets \
      -d name=events -d ramQuotaMB=1500 -d authType=none -d replicaNumber=0 -d bucketType=couchbase

fg 1
