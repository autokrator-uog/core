// Modified version of original StreamConsumer from rdkafka library by fede1024 to
// work within Actix actors. Modifications by David Wood. Original by Federico Giraud.
//
// Link to file:
// https://github.com/fede1024/rust-rdkafka/blob/master/src/consumer/stream_consumer.rs
//
// Original license:
// MIT License
//
// Copyright (c) 2016 Federico Giraud
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use futures::{Future, Poll, Sink, Stream};
use futures::sync::mpsc;
use rdkafka::config::{FromClientConfig, FromClientConfigAndContext, ClientConfig};
use rdkafka::consumer::base_consumer::BaseConsumer;
use rdkafka::consumer::{Consumer, ConsumerContext, EmptyConsumerContext};
use rdkafka::error::{KafkaError, KafkaResult};
use rdkafka::message::OwnedMessage;
use rdkafka::util::duration_to_millis;

/// **Note:** This is a modified version of the rdkafka `MessageStream` that produces
/// `OwnedMessage` instead of `BorrowedMessage`, this is important as it allows us to eliminate
/// the lifetime that stops the consumer from working within a actor.
///
/// It can be used to receive messages as they are consumed from Kafka. Note: there might be
/// buffering between the actual Kafka consumer and the receiving end of this stream, so it is not
/// advised to use automatic commit, as some messages might have been consumed by the internal Kafka
/// consumer but not processed. Manual offset storing should be used, see the `store_offset`
/// function on `Consumer`.
pub struct MessageStream<C: ConsumerContext + 'static> {
    _consumer: StreamConsumer<C>,
    receiver: mpsc::Receiver<Option<KafkaResult<OwnedMessage>>>,
}

impl<C: ConsumerContext + 'static> MessageStream<C> {
    fn new(consumer: StreamConsumer<C>, receiver: mpsc::Receiver<Option<KafkaResult<OwnedMessage>>>) -> MessageStream<C> {
        MessageStream {
            _consumer: consumer,
            receiver: receiver,
        }
    }
}

impl<C: ConsumerContext> Stream for MessageStream<C> {
    type Item = KafkaResult<OwnedMessage>;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.receiver.poll().map(|ready| ready.map(|option| option.map(|owned_message_opt|
            owned_message_opt.map_or(Err(KafkaError::NoMessageReceived),
                                     |owned_message| owned_message))))
    }
}

/// Internal consumer loop. This is the main body of the thread that will drive the stream consumer.
/// If `send_none` is true, the loop will send a None into the sender every time the poll times out.
fn poll_loop<C: ConsumerContext>(
    consumer: Arc<BaseConsumer<C>>,
    sender: mpsc::Sender<Option<KafkaResult<OwnedMessage>>>,
    should_stop: Arc<AtomicBool>,
    poll_interval: Duration,
    send_none: bool,
) {
    trace!("polling thread loop started");
    let mut curr_sender = sender;
    let poll_interval_ms = duration_to_millis(poll_interval) as i32;

    while !should_stop.load(Ordering::Relaxed) {
        trace!("polling base consumer");
        let future_sender = match consumer.poll(poll_interval_ms) {
            None => {
                if send_none {
                    curr_sender.send(None)
                } else {
                    continue
                }
            },
            Some(result_message) => curr_sender.send(Some(result_message.map(|m| m.detach()))),
        };
        match future_sender.wait() {
            Ok(new_sender) => curr_sender = new_sender,
            Err(e) => {
                debug!("sender not available: sender='{}'", e);
                break;
            }
        };
    }

    trace!("polling thread loop terminated");
}

/// **Note:** This is a modified version of the rdkafka `StreamConsumer` that produces
/// `OwnedMessage` instead of `BorrowedMessage`, this is important as it allows us to eliminate
/// the lifetime that stops the consumer from working within a actor. However, a given instance of
/// `StreamConsumer` will only work for a single stream.
///
/// A Kafka Consumer providing a `futures::Stream` interface.
///
/// This consumer doesn't need to be polled since it has a separate polling thread. Due to the
/// asynchronous nature of the stream, some messages might be consumed by the consumer without being
/// processed on the other end of the stream. If auto commit is used, it might cause message loss
/// after consumer restart. Manual offset storing should be used, see the `store_offset` function on
/// `Consumer`.
#[must_use = "Consumer polling thread will stop immediately if unused"]
pub struct StreamConsumer<C: ConsumerContext + 'static> {
    consumer: Arc<BaseConsumer<C>>,
    should_stop: Arc<AtomicBool>,
    handle: Cell<Option<JoinHandle<()>>>,
}

impl<C: ConsumerContext> Consumer<C> for StreamConsumer<C> {
    fn get_base_consumer(&self) -> &BaseConsumer<C> {
        Arc::as_ref(&self.consumer)
    }
}

impl FromClientConfig for StreamConsumer<EmptyConsumerContext> {
    fn from_config(config: &ClientConfig) -> KafkaResult<StreamConsumer<EmptyConsumerContext>> {
        StreamConsumer::from_config_and_context(config, EmptyConsumerContext)
    }
}

/// Creates a new `StreamConsumer` starting from a `ClientConfig`.
impl<C: ConsumerContext> FromClientConfigAndContext<C> for StreamConsumer<C> {
    fn from_config_and_context(config: &ClientConfig, context: C) -> KafkaResult<StreamConsumer<C>> {
        let stream_consumer = StreamConsumer {
            consumer: Arc::new(BaseConsumer::from_config_and_context(config, context)?),
            should_stop: Arc::new(AtomicBool::new(false)),
            handle: Cell::new(None),
        };
        Ok(stream_consumer)
    }
}

impl<C: ConsumerContext> StreamConsumer<C> {
    /// Starts the StreamConsumer with default configuration (100ms polling interval and no
    /// `NoMessageReceived` notifications).
    pub fn start(self) -> MessageStream<C> {
        self.start_with(Duration::from_millis(100), false)
    }

    /// Starts the StreamConsumer with the specified poll interval. Additionally, if
    /// `no_message_error` is set to true, it will return an error of type
    /// `KafkaError::NoMessageReceived` every time the poll interval is reached and no message has
    /// been received.
    pub fn start_with(self, poll_interval: Duration, no_message_error: bool) -> MessageStream<C> {
        let (sender, receiver) = mpsc::channel(0);
        let consumer = self.consumer.clone();
        let should_stop = self.should_stop.clone();
        let handle = thread::Builder::new()
            .name("poll".to_string())
            .spawn(move || {
                poll_loop(consumer, sender, should_stop, poll_interval, no_message_error);
            })
            .expect("failed to start polling thread");
        self.handle.set(Some(handle));
        MessageStream::new(self, receiver)
    }

    /// Stops the StreamConsumer, blocking the caller until the internal consumer has been stopped.
    pub fn stop(&self) {
        if let Some(handle) = self.handle.take() {
            trace!("stopping polling");
            self.should_stop.store(true, Ordering::Relaxed);
            match handle.join() {
                Ok(()) => trace!("polling stopped"),
                Err(e) => warn!("failure while terminating thread: error='{:?}'", e),
            };
        }
    }
}

impl<C: ConsumerContext> Drop for StreamConsumer<C> {
    fn drop(&mut self) {
        trace!("destroy StreamConsumer");
        // The polling thread must be fully stopped before we can proceed with the actual drop,
        // otherwise it might consume from a destroyed consumer.
        self.stop();
    }
}
