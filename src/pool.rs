use std::fmt;

use futures::sync::mpsc::{channel, Sender, SendError};
use futures::sink::Sink;
use futures::stream::Stream;
use futures::future::Future;
use futures::{StartSend, Poll};
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
        let fut = rx.map_err(|_| -> E {
            // channel receive never fails
            unreachable!();
        }).forward(multiplexer).map_err(|e| {
            error!("Connection pool crashed with error: {}", e);
        }).map(|(_sink, _stream)| ());
        handle.spawn(fut);
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
