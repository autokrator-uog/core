mod stream;

use actix::{Actor, Address, AsyncContext, Context, Handler, StreamHandler, Response, ResponseType};
use failure::{Error, ResultExt};
use futures::stream::Stream;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::Consumer as ConsumerTrait;
use rdkafka::message::OwnedMessage;

use bus::Bus;
use error::ErrorKind;
use consumer::stream::StreamConsumer;

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
    pub fn start(brokers: &str, group: &str, topic: &str,
                 bus: Address<Bus>) -> Result<(), Error> {
        info!("starting Kafka listener: brokers={} group={}", brokers, group);
        let consumer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("group.id", group)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "false")
            .create::<StreamConsumer<_>>()
            .context(ErrorKind::KafkaConsumerCreationFailure)?;
        info!("subscribing to topic on Kafka listener: topic={}", topic);
        consumer.subscribe(&[topic]).context(ErrorKind::KafkaConsumerSubscriptionFailure)?;

        let _: () = Consumer::create(move |ctx| {
            ctx.add_stream(consumer.start()
                .filter_map(|result| {
                    match result {
                        Ok(m) => Some(m),
                        Err(e) => {
                            error!("kafka error: {:?}", e);
                            None
                        }
                    }
                }).map(|msg| {
                    KafkaMessage(msg)
                }).map_err(|_| {
                    Error::from(ErrorKind::KafkaErrorReceived)
                }));

            Consumer { bus: bus }
        });

        Ok(())
    }
}

impl Actor for Consumer {
    type Context = Context<Self>;
}

// By implementing StreamHandler, we can add streams to this actor which will trigger the
// event functions below.
impl StreamHandler<KafkaMessage, Error> for Consumer {
    fn started(&mut self, _ctx: &mut Context<Self>) { info!("consumer listener started"); }
    fn finished(&mut self, _ctx: &mut Context<Self>) { info!("consumer listener finished"); }
}

impl Handler<KafkaMessage, Error> for Consumer {
    /// Handle an incoming message from Kafka.
    fn handle(&mut self, msg: KafkaMessage, _: &mut Context<Self>) -> Response<Self, KafkaMessage> {
        info!("received message from Kafka: {:?}", msg);
        // No need for inter-actor communication so we can return a unit response.
        Self::empty()
    }
}
