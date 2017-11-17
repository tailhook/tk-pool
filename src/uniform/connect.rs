use std::net::SocketAddr;

use futures::{Future, Async, Sink};

use uniform::{FutureOk, FutureErr};


pub(crate) struct ConnectFuture<F> {
    addr: SocketAddr,
    future: F,
}

impl<F: Future> ConnectFuture<F> {
    pub fn new(addr: SocketAddr, future: F) -> ConnectFuture<F> {
        ConnectFuture { addr, future }
    }
}

impl<F: Future> Future for ConnectFuture<F>
    where F::Item: Sink,
{
    type Item = FutureOk<F::Item>;
    type Error = FutureErr<F::Error, <F::Item as Sink>::SinkError>;
    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        self.future.poll()
        .map(|ready| ready.map(|s| FutureOk::Connected(self.addr, s)))
        .map_err(|e| FutureErr::CantConnect(self.addr, e))
    }
}
