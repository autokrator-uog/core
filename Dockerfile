FROM rust:1.21

WORKDIR /usr/src/app
COPY . .

RUN cargo install

ENV LOG_LEVEL debug
ENV BIND 0.0.0.0:8081

CMD event-bus --broker $BROKER -g $GROUP -l $LOG_LEVEL -b $BIND server -t $TOPIC
