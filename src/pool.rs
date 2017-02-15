use std::fmt;

use futures::sync::mpsc::{channel, Sender, SendError};
use futures::sink::Sink;
use futures::stream::{Stream, Fuse};
use futures::future::Future;
use futures::{StartSend, Poll, Async, AsyncSink};
use tokio_core::reactor::Handle;


/// Pool is an object that is convenient to use for client connections. It
/// will spawn a future that processes requests and establish a channel to it.
/// It can be used from multiple threads (but will establish connections in
/// an the one that originally created pool).
///
/// It's still a `Sink` and you are free to implement `Service` or whatever
/// high level interfaces apply for your protocol.
pub struct Pool<M> {
    channel: Sender<M>,
}

// This is similar to `futures::stream::Forward` but also calls poll_complete
// on wakeups. This is important to keep connection pool up to date when
// no new requests are coming in.
struct Forwarder<T: Stream, K: Sink<SinkItem=T::Item>> {
    source: Fuse<T>,
    sink: K,
    buffered: Option<T::Item>,
}

impl<M> Pool<M> {

    //! Create a connection pool the specified buffer size and specified Sink
    //! (which should be a Multiplexer)
    //!
    //! This is basicaly (sans the error conversion):
    //!
    //! ```rust,ignore
    //! let (tx, rx) = channel(buffer_size);
    //! handle.spawn(rx.forward(multiplexer));
    //! return tx;
    //! ```
    //!
    //!
    // TODO(tailhook) should E be Error?
    pub fn create<S, E>(handle: &Handle, buffer_size: usize, multiplexer: S)
        -> Pool<M>
        where E: fmt::Display,
              S: Sink<SinkItem=M, SinkError=E> + 'static,
              M: 'static, // TODO(tailhook) should this bound be on type?
    {
        let (tx, rx) = channel(buffer_size);
        handle.spawn(Forwarder {
            source: rx.fuse(),
            sink: multiplexer,
            buffered: None,
        });
        return Pool {
            channel: tx
        };
    }

}

impl<M> Sink for Pool<M> {
    type SinkItem=M;
    // TODO(tailhook) should we have some custom error for that?
    type SinkError=SendError<M>;

    fn start_send(&mut self, item: Self::SinkItem)
        -> StartSend<Self::SinkItem, Self::SinkError>
    {
        self.channel.start_send(item)
        // SendError contains actual object, but it also notifies us that
        // receiver is closed.
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
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

impl<M> Clone for Pool<M> {
    fn clone(&self) -> Self {
        Pool {
            channel: self.channel.clone(),
        }
    }
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
