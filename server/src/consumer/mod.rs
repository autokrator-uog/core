mod stream;

use std::str::from_utf8;

use actix::{Actor, Address, Context, ResponseType, StreamHandler};
use common::schemas::Event;
use failure::{Error, ResultExt};
use futures::stream::Stream;
use rdkafka::Message;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::Consumer as ConsumerTrait;
use rdkafka::message::OwnedMessage;
use serde_json::{from_str, to_string_pretty};

use bus::Bus;
use consumer::stream::StreamConsumer;
use error::ErrorKind;
use signals;

/// `KafkaMessage` is a wrapper type that allows us to implement `ResponseType` for Kafka's
/// OwnedMessage. It is created by the `Codec` encoder/decoder.
#[derive(Debug)]
struct KafkaMessage(pub OwnedMessage);

impl ResponseType for KafkaMessage {
    type Item = ();
    type Error = ();
}

/// The consumer actor handles incoming messages from Kafka and forwards them using the correct
/// message on the Bus.
pub struct Consumer {
    bus: Address<Bus>
}

impl Consumer {
    /// Start the Kafka listener given the arguments for the `server` subcommand.
    pub fn launch(brokers: &str, group: &str, topic: &str,
                 bus: Address<Bus>) -> Result<(), Error> {
        info!("starting kafka listener: brokers='{}' group='{}'", brokers, group);
        let consumer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("group.id", group)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "false")
            .create::<StreamConsumer<_>>()
            .context(ErrorKind::KafkaConsumerCreation)?;
        info!("subscribing to topic on kafka listener: topic='{}'", topic);
        consumer.subscribe(&[topic]).context(ErrorKind::KafkaConsumerSubscription)?;

        let _: () = Self::create(move |ctx| {
            Self::add_stream(consumer.start()
                .filter_map(|result| {
                    match result {
                        Ok(m) => Some(m),
                        Err(e) => {
                            error!("kafka stream: error='{}'", e);
                            None
                        }
                    }
                }).map(|msg| {
                    KafkaMessage(msg)
                }).map_err(|_| {
                    Error::from(ErrorKind::KafkaErrorReceived)
                }), ctx);

            Self { bus: bus }
        });

        Ok(())
    }

    fn process_message(&mut self, message: KafkaMessage) -> Result<(), Error> {
        debug!("starting processing message from kafka");
        let contents = self.get_message_contents(message)?;

        let parsed: Event = from_str(&contents).context(ErrorKind::ParseJsonFromKafka)?;
        debug!("parsed message from kafka");
        info!("received message on kafka: message=\n{}",
              to_string_pretty(&parsed).context(ErrorKind::SerializeJsonForSending)?);

        let message = Event {
            message_type: Some("event".to_owned()),
            ..parsed.clone()
        };

        self.bus.send(signals::PropagateEvent { event: message });
        debug!("finished processing message from kafka");
        Ok(())
    }

    fn get_message_contents(&mut self, message: KafkaMessage) -> Result<String, Error> {
        let message = message.0;

        debug!("retrieving payload from kafka message");
        let payload = message.payload().ok_or(ErrorKind::KafkaMessageWithNoPayload)?;

        debug!("converting payload to string");
        let converted = from_utf8(payload).context(ErrorKind::ParseBytesAsUtf8)?;

        Ok(converted.to_string())
    }
}

impl Actor for Consumer {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) { info!("consumer listener started"); }
    fn stopped(&mut self, _ctx: &mut Context<Self>) { info!("consumer listener finished"); }
}

impl StreamHandler<KafkaMessage, Error> for Consumer {
    /// Handle an incoming message from Kafka.
    fn handle(&mut self, message: KafkaMessage, _ctx: &mut Context<Self>) {
        if let Err(e) = self.process_message(message) {
            error!("processing message from kafka: error='{}'", e);
        }
    }
}
