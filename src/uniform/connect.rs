use futures::{Future, Async};

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

impl<F: Future> Future for ConnectFuture<F> {
    type Item = FutureOk;
    type Error = FutureErr;
    fn poll(&mut self) -> Result<Async<FutureOk>, FutureErr> {
        unimplemented!();
    }
}
