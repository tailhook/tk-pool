use std::fmt;

use futures::sync::mpsc::{channel, Sender, SendError};
use futures::sink::Sink;
use futures::stream::{Stream, Fuse};
use futures::future::Future;
use futures::{StartSend, Poll, Async, AsyncSink};
use tokio_core::reactor::Handle;

use metrics::Collect;
use config::{NewQueue, Queue, DefaultQueue};


/// Pool is an object you use to access a connection pool
///
/// Usually the whole logic of connection pool is spawned in another future.
/// This object encapsulates a channel that is used to communicate with pool.
/// This object also contains a clone of metrics collection object as it's
/// very important to collect metrics at this side of a channel.
#[derive(Debug)]
pub struct Pool<V, M> {
    channel: Sender<V>,
    metrics: M,
}

impl<I, M> NewQueue<I, M> for DefaultQueue {
    type Pool = Pool<I, M>;
    fn construct(self, metrics: M) -> Pool<I, M> {
        Queue(100).construct(metrics)
    }
}

impl<I, M> NewQueue<I, M> for Queue {
    type Pool = Pool<I, M>;
    fn construct(self, metrics: M) -> Pool<I, M> {
        let (tx, rx) = channel(self.0);
        return Pool {
            channel: tx,
            metrics: metrics,
        }
    }
}


trait AssertTraits: Clone + Send + Sync {}
impl<V: Send, M: Collect> AssertTraits for Pool<V, M> {}

impl<V, M: Clone> Clone for Pool<V, M> {
    fn clone(&self) -> Self {
        Pool {
            channel: self.channel.clone(),
            metrics: self.metrics.clone(),
        }
    }
}


impl<V, M> Sink for Pool<V, M> {
    type SinkItem=V;
    // TODO(tailhook) make own error
    type SinkError=SendError<V>;

    fn start_send(&mut self, item: Self::SinkItem)
        -> StartSend<Self::SinkItem, Self::SinkError>
    {
        // TODO(tailhook) metrics
        // TODO(tailhook) turn error into NotReady, and flag closed
        self.channel.start_send(item)
        // SendError contains actual object, but it also notifies us that
        // receiver is closed.
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        // TODO(tailhook) turn closed flag into error
        self.channel.poll_complete()
        .map_err(|_| {
            // In fact poll_complete of the channel does nothing for now
            // Even if this is fixed there is no sense to return error for
            // it because error contains a value SinkItem and there is no way
            // to construct a value from nothing
            unreachable!();
        })
    }
}

/*
// This is similar to `futures::stream::Forward` but also calls poll_complete
// on wakeups. This is important to keep connection pool up to date when
// no new requests are coming in.
struct Forwarder<T: Stream, K: Sink<SinkItem=T::Item>> {
    source: Fuse<T>,
    sink: K,
    buffered: Option<T::Item>,
}

impl<T: Stream<Error=()>, K: Sink<SinkItem=T::Item>> Future for Forwarder<T, K>
    where K::SinkError: fmt::Display,
{
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Result<Async<()>, ()> {
        // If we've got an item buffered already, we need to write it to the
        // sink before we can do anything else
        if let Some(item) = self.buffered.take() {
            let res = self.sink.start_send(item)
                .map_err(|e| error!("Pool output error: {}. \
                                     Stopping pool.", e))?;
            match res {
                AsyncSink::NotReady(item) => {
                    self.buffered = Some(item);
                    return Ok(Async::NotReady);
                }
                AsyncSink::Ready => {}
            }
        }

        loop {
            let res = self.source.poll()
                .map_err(|()| error!("Pool input aborted. Stopping."))?;
            match res {
                Async::Ready(Some(item)) => {
                    let res = self.sink.start_send(item)
                        .map_err(|e| error!("Pool output error: {}. \
                                             Stopping pool.", e))?;
                    match res {
                        AsyncSink::NotReady(item) => {
                            self.buffered = Some(item);
                            break;
                        }
                        AsyncSink::Ready => {}
                    }
                }
                Async::Ready(None) => {
                    let res = self.sink.poll_complete()
                        .map_err(|e| error!("Pool output error: {}. \
                            Stopping pool.", e))?;
                    match res {
                        Async::Ready(()) => return Ok(Async::Ready(())),
                        Async::NotReady => return Ok(Async::NotReady),
                    }
                }
                Async::NotReady => {
                    self.sink.poll_complete()
                        .map_err(|e| error!("Pool output error: {}. \
                            Stopping pool.", e))?;
                    break;
                }
            }
        }
        Ok(Async::NotReady)
    }
}
*/
