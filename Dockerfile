FROM rust:1.23-jessie

RUN apt-get update \
    && apt-get install -y libev4 libssl1.0.0 cmake build-essential \
    && wget https://github.com/couchbase/libcouchbase/releases/download/2.8.1/libcouchbase-2.8.1_jessie_amd64.tar \
    && tar xf libcouchbase-2.8.1_jessie_amd64.tar \
    && dpkg -i libcouchbase-2.8.1_jessie_amd64/*.deb

WORKDIR /usr/src/app
COPY . .

RUN RUST_BACKTRACE=1 cargo install

ENV LOG_LEVEL debug
ENV BIND 0.0.0.0:8081

CMD event-bus --broker $BROKER -g $GROUP -l $LOG_LEVEL -b $BIND server -t $TOPIC
