use std::net::SocketAddr;
use std::marker::PhantomData;

use futures::{Future, Async, Sink};

use uniform::{FutureOk, FutureErr};


pub(crate) struct SinkFuture<S, E> {
    addr: SocketAddr,
    sink: S,
    phantom: PhantomData<*const E>,
}

impl<S: Sink, E> SinkFuture<S, E> {
    pub fn new(addr: SocketAddr, sink: S) -> SinkFuture<S, E> {
        SinkFuture { addr, sink, phantom: PhantomData }
    }
}

impl<S: Sink, E> Future for SinkFuture<S, E> {
    type Item = FutureOk<S>;
    type Error = FutureErr<E, S::SinkError>;
    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        unimplemented!();
    }
}
