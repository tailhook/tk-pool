use futures::{Future, Async, Sink};

use uniform::{FutureOk, FutureErr};
use uniform::chan::Helper;


pub(in uniform) struct ConnectFuture<F>
    where F: Future,
          F::Item: Sink,
{
    task: Option<Helper<<F::Item as Sink>::SinkItem>>,
    future: F,
}

impl<F: Future> ConnectFuture<F>
    where F: Future,
          F::Item: Sink,
{
    pub fn new(task: Helper<<F::Item as Sink>::SinkItem>, future: F)
        -> ConnectFuture<F>
    {
        ConnectFuture { task: Some(task), future }
    }
}

impl<F: Future> Future for ConnectFuture<F>
    where F::Item: Sink,
{
    type Item = FutureOk<F::Item>;
    type Error = FutureErr<F::Error, <F::Item as Sink>::SinkError>;
    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        let snk = {
            let task = self.task.as_ref().expect("poll invariant");
            match task.poll_close() {
                Async::Ready(()) => {
                    return Ok(Async::Ready(FutureOk::Aborted(task.addr())));
                }
                Async::NotReady => {}
            }
            match self.future.poll() {
                Ok(Async::Ready(s)) => s,
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(e) => return Err(FutureErr::CantConnect(task.addr(), e)),
            }
        };
        let task = self.task.take().expect("poll invariant");
        Ok(Async::Ready(FutureOk::Connected(task, snk)))
    }
}
