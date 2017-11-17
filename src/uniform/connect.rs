use futures::{Future, Async, Sink};

use uniform::{FutureOk, FutureErr};


pub(crate) struct ConnectFuture<F> {
    future: F,
}

impl<F: Future> ConnectFuture<F> {
    pub fn new(f: F) -> ConnectFuture<F> {
        ConnectFuture {
            future: f,
        }
    }
}

impl<F: Future> Future for ConnectFuture<F>
    where F::Item: Sink,
{
    type Item = FutureOk<F::Error, <F::Item as Sink>::SinkError>;
    type Error = FutureErr<F::Error, <F::Item as Sink>::SinkError>;
    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        unimplemented!();
    }
}
