//! A queue (buffer) of requests sent to connection pool
use std::fmt;
use futures::{AsyncSink, Stream, StartSend, Poll, Async};
use futures::sync::mpsc::{self, channel, Sender};
use futures::sink::Sink;
use futures::stream::Fuse;
use futures::future::Future;
use tokio_core::reactor::Handle;

use metrics::Collect;
use error_log::{ErrorLog, ShutdownReason};
use config::{Queue, DefaultQueue, private};


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

/// Error returned by the sink, when underlying pool is closed
///
/// The error contains underlying item that was sent using `start_send`
pub struct QueueError<V>(V);


/// This is similar to `Forward` from `futures` but has metrics and errors
#[derive(Debug)]
#[must_use = "futures do nothing unless polled"]
struct ForwardFuture<S, M, E>
    where S: Sink
{
     receiver: Fuse<mpsc::Receiver<S::SinkItem>>,
     buffer: Option<S::SinkItem>,
     metrics: M,
     errors: E,
     sink: S,
}

impl<I: 'static, M> private::NewQueue<I, M> for DefaultQueue {
    type Pool = Pool<I, M>;
    fn spawn_on<S, E>(self, pool: S, err: E, metrics: M, handle: &Handle)
        -> Self::Pool
        where S: Sink<SinkItem=I, SinkError=private::Done> + 'static,
              E: ErrorLog + 'static,
              M: Collect + 'static,
    {
        Queue(100).spawn_on(pool, err, metrics, handle)
    }
}

impl<I: 'static, M> private::NewQueue<I, M> for Queue {
    type Pool = Pool<I, M>;
    fn spawn_on<S, E>(self, pool: S, e: E, metrics: M, handle: &Handle)
        -> Self::Pool
        where S: Sink<SinkItem=I, SinkError=private::Done> + 'static,
              E: ErrorLog + 'static,
              M: Collect + 'static,
    {
        // one item is buffered ForwardFuture
        let buf_size = self.0.saturating_sub(1);
        let (tx, rx) = channel(buf_size);
        handle.spawn(ForwardFuture {
            receiver: rx.fuse(),
            metrics: metrics.clone(),
            errors: e,
            sink: pool,
            buffer: None,
        });
        return Pool {
            channel: tx,
            metrics,
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

impl<S, M, E> ForwardFuture<S, M, E>
    where S: Sink<SinkError=private::Done>,
          M: Collect,
          E: ErrorLog,
{
    fn poll_forever(&mut self) -> Async<()> {
        if let Some(item) = self.buffer.take() {
            match self.sink.start_send(item) {
                Ok(AsyncSink::Ready) => {
                    self.metrics.request_forwarded();
                }
                Ok(AsyncSink::NotReady(item)) => {
                    self.buffer = Some(item);
                    return Async::NotReady;
                }
                Err(private::Done) => return Async::Ready(()),
            }
        }

        let was_done = self.receiver.is_done();
        loop {
            match self.receiver.poll() {
                Ok(Async::Ready(Some(item))) => {
                    match self.sink.start_send(item) {
                        Ok(AsyncSink::Ready) => {
                            self.metrics.request_forwarded();
                            continue;
                        }
                        Ok(AsyncSink::NotReady(item)) => {
                            self.buffer = Some(item);
                            return Async::NotReady;
                        }
                        Err(private::Done) => return Async::Ready(()),
                    }
                }
                Ok(Async::Ready(None)) => {
                    if !was_done {
                        self.errors.pool_shutting_down(
                            ShutdownReason::RequestStreamClosed);
                    }
                    match self.sink.close() {
                        Ok(Async::NotReady) => {
                            return Async::NotReady;
                        }
                        Ok(Async::Ready(())) | Err(private::Done) => {
                            return Async::Ready(());
                        }
                    }
                }
                Ok(Async::NotReady) => match self.sink.poll_complete() {
                    Ok(_) => {
                        return Async::NotReady;
                    }
                    Err(private::Done) => {
                        return Async::Ready(());
                    }
                }
                // No errors in channel receiver
                Err(()) => unreachable!(),
            }
        }
    }
}

impl<S, M, E> Future for ForwardFuture<S, M, E>
    where S: Sink<SinkError=private::Done>,
          M: Collect,
          E: ErrorLog,
{
    type Item = ();
    type Error = ();  // Really Void
    fn poll(&mut self) -> Result<Async<()>, ()> {
        match self.poll_forever() {
            Async::NotReady => Ok(Async::NotReady),
            Async::Ready(()) => {
                self.errors.pool_closed();
                self.metrics.pool_closed();
                Ok(Async::Ready(()))
            }
        }
    }
}


impl<V, M> Sink for Pool<V, M>
    where M: Collect,
{
    type SinkItem=V;
    type SinkError=QueueError<V>;

    fn start_send(&mut self, item: Self::SinkItem)
        -> StartSend<Self::SinkItem, Self::SinkError>
    {
        match self.channel.start_send(item) {
            Ok(AsyncSink::Ready) => {
                self.metrics.request_queued();
                Ok(AsyncSink::Ready)
            }
            Ok(AsyncSink::NotReady(item)) => Ok(AsyncSink::NotReady(item)),
            Err(e) => Err(QueueError(e.into_inner())),
        }
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
    fn close(&mut self) -> Poll<(), Self::SinkError> {
        self.channel.close()
        .map_err(|_| {
            // In fact close of the channel does nothing for now
            // Even if this is fixed there is no sense to return error for
            // it because error contains a value SinkItem and there is no way
            // to construct a value from nothing
            unreachable!();
        })
    }
}

impl<T> QueueError<T> {
    /// Return ownership of contained message
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> fmt::Display for QueueError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("connection pool is closed")
    }
}

impl<T> fmt::Debug for QueueError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("QueueError(_)")
    }
}

impl<T> ::std::error::Error for QueueError<T> {
    fn description(&self) -> &str {
        "QueueError"
    }
    fn cause(&self) -> Option<&::std::error::Error> {
        None
    }
}
