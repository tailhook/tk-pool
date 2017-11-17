use std::net::SocketAddr;
use std::marker::PhantomData;

use futures::{Future, Async, Sink, AsyncSink};

use uniform::{FutureOk, FutureErr};
use uniform::chan::{Action, Receiver};


pub(crate) struct SinkFuture<S, E>
    where S: Sink,
{
    addr: SocketAddr,
    sink: S,
    recv: Receiver<S::SinkItem>,
    phantom: PhantomData<*const E>,
}

impl<S: Sink, E> SinkFuture<S, E> {
    pub fn new(addr: SocketAddr, sink: S, recv: Receiver<S::SinkItem>)
        -> SinkFuture<S, E>
    {
        SinkFuture { addr, sink, recv: recv, phantom: PhantomData }
    }
}

impl<S: Sink, E> Future for SinkFuture<S, E> {
    type Item = FutureOk<S>;
    type Error = FutureErr<E, S::SinkError>;
    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        match self.recv.take() {
            Action::StartSend(item) => match self.sink.start_send(item) {
                Ok(AsyncSink::Ready) => {
                    self.recv.clear_backpressure();
                    Ok(Async::NotReady)
                }
                Ok(AsyncSink::NotReady(item)) => {
                    self.recv.backpressure(item);
                    Ok(Async::NotReady)
                }
                Err(e) => {
                    Err(FutureErr::Disconnected(self.addr, e))
                }
            }
            Action::Poll => match self.sink.poll_complete() {
                Ok(Async::NotReady) => {
                    self.recv.set_backpressure();
                    Ok(Async::NotReady)
                }
                Ok(Async::Ready(())) => {
                    self.recv.clear_backpressure();
                    Ok(Async::NotReady)
                }
                Err(e) => {
                    Err(FutureErr::Disconnected(self.addr, e))
                }
            }
        }
    }
}
