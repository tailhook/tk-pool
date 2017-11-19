use std::marker::PhantomData;

use futures::{Future, Async, Sink, AsyncSink};

use uniform::{FutureOk, FutureErr};
use uniform::chan::{Action, Helper};


pub(in uniform) struct SinkFuture<S, E>
    where S: Sink,
{
    sink: S,
    task: Helper<S::SinkItem>,
    phantom: PhantomData<*const E>,
}

impl<S: Sink, E> SinkFuture<S, E> {
    pub fn new(sink: S, task: Helper<S::SinkItem>)
        -> SinkFuture<S, E>
    {
        SinkFuture { sink, task, phantom: PhantomData }
    }
}

impl<S: Sink, E> Future for SinkFuture<S, E> {
    type Item = FutureOk<S>;
    type Error = FutureErr<E, S::SinkError>;
    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        match self.task.take() {
            Action::StartSend(item) => match self.sink.start_send(item) {
                Ok(AsyncSink::Ready) => {
                    Ok(Async::NotReady)
                }
                Ok(AsyncSink::NotReady(item)) => {
                    self.task.backpressure(item);
                    Ok(Async::NotReady)
                }
                Err(e) => {
                    self.task.closed();
                    Err(FutureErr::Disconnected(self.task.addr(), e))
                }
            }
            Action::Poll => match self.sink.poll_complete() {
                Ok(_)  => {
                    // By contract there is no difference in Ready and NotReady
                    // I.e. both of them may mean that another element can
                    // be pushed now. They only distinquish whether there is
                    // something left in the buffer.
                    self.task.requeue();
                    Ok(Async::NotReady)
                }
                Err(e) => {
                    self.task.closed();
                    Err(FutureErr::Disconnected(self.task.addr(), e))
                }
            }
            Action::Close => match self.sink.close() {
                Ok(Async::Ready(()))  => {
                    Ok(Async::Ready(FutureOk::Closed(self.task.addr())))
                }
                Ok(Async::NotReady)  => Ok(Async::NotReady),
                Err(e) => {
                    self.task.closed();
                    Err(FutureErr::Disconnected(self.task.addr(), e))
                }
            }
        }
    }
}
