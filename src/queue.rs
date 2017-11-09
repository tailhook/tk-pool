use futures::sync::mpsc::{self, channel, Sender, SendError};
use futures::sink::Sink;
use futures::stream::{Stream};
use futures::future::Future;
use futures::{StartSend, Poll, Async};
use tokio_core::reactor::Handle;
use void::{Void, unreachable};

use metrics::Collect;
use config::{NewQueue, Queue, DefaultQueue, ErrorLog};


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


/// Wraps mpsc receiver but has a Void error (as real receiver too)
#[derive(Debug)]
pub struct Receiver<T> {
     channel: mpsc::Receiver<T>,
}

impl<T> Stream for Receiver<T> {
    type Item = T;
    type Error = Void;
    fn poll(&mut self) -> Result<Async<Option<T>>, Void> {
        self.channel.poll().map_err(|()| unreachable!())
    }
}

impl<I: 'static, M> NewQueue<I, M> for DefaultQueue {
    type Pool = Pool<I, M>;
    fn spawn_on<S, E>(self, pool: S, err: E, metrics: M, handle: &Handle)
        -> Self::Pool
        where S: Sink<SinkItem=I, SinkError=Void> + 'static,
              E: ErrorLog + 'static,
    {
        Queue(100).spawn_on(pool, err, metrics, handle)
    }
}

impl<I: 'static, M> NewQueue<I, M> for Queue {
    type Pool = Pool<I, M>;
    fn spawn_on<S, E>(self, pool: S, e: E, metrics: M, handle: &Handle)
        -> Self::Pool
        where S: Sink<SinkItem=I, SinkError=Void> + 'static,
              E: ErrorLog + 'static,
    {
        let (tx, rx) = channel(self.0);
        handle.spawn(
            Receiver {
                channel: rx,
            }.forward(pool)
            .map(move |(_, _)| e.connection_pool_shut_down())
            .map_err(|e| unreachable(e)));
        return Pool {
            channel: tx,
            metrics: metrics,
        };
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
