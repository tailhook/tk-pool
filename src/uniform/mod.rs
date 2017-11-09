mod failures;
mod aligner;

use std::time::Duration;

use futures::{Future, Async, Sink, AsyncSink};
use tokio_core::reactor::Handle;

use config::{ErrorLog, NewMux};
use connect::Connect;
use metrics::Collect;
use void::{Void};
use uniform::aligner::Aligner;
use uniform::failures::Blacklist;


/// A constructor for a uniform connection pool with lazy connections
pub struct LazyUniform {
    pub(crate) conn_limit: u32,
    pub(crate) reconnect_timeout: Duration,
}

pub struct Lazy<C, E, M> {
    conn_limit: u32,
    reconnect_ms: (u64, u64),  // easier to make random value
    connector: C,
    errors: E,
    metrics: M,
    aligner: Aligner,
    blist: Blacklist,
}

impl<C, E, M> NewMux<C, E, M> for LazyUniform
    where C: Connect + 'static,
          <<C as Connect>::Future as Future>::Item: Sink,
          E: ErrorLog<
            ConnectionError=<C::Future as Future>::Error,
            SinkError=<<C::Future as Future>::Item as Sink>::SinkError,
            >,
          E: 'static,
          M: Collect + 'static,
{
    type Sink = Lazy<C, E, M>;
    fn construct(self,
        h: &Handle, connector: C, errors: E, metrics: M)
        -> Lazy<C, E, M>
    {
        let reconn_ms = self.reconnect_timeout.as_secs() * 1000 +
            (self.reconnect_timeout.subsec_nanos() / 1000_000) as u64;
        Lazy {
            conn_limit: self.conn_limit,
            reconnect_ms: (reconn_ms / 2, reconn_ms * 3 / 2),
            blist: Blacklist::new(h),
            aligner: Aligner::new(),
            connector, errors, metrics,
        }
    }
}

impl<C, E, M> Lazy<C, E, M>
    where C: Connect,
          E: ErrorLog<ConnectionError=<C::Future as Future>::Error>,
          M: Collect,
{

    fn process_requests(&mut self) {
        unimplemented!();
    }
}

impl<C, E, M> Sink for Lazy<C, E, M>
    where C: Connect,
          <C::Future as Future>::Item: Sink,
          E: ErrorLog<
            ConnectionError=<C::Future as Future>::Error,
            SinkError=<<C::Future as Future>::Item as Sink>::SinkError>,
          M: Collect,
{
    type SinkItem = <<C::Future as Future>::Item as Sink>::SinkItem;
    type SinkError = Void;
    fn start_send(&mut self, v: Self::SinkItem)
        -> Result<AsyncSink<Self::SinkItem>, Void>
    {
        unimplemented!()
        /*
        loop {
            self.process_requests();
            // TODO(tailhook) if has capacity
            match self.requests.poll() {
                Err(e) => unreachable(e),
                Ok(Async::Ready(Some(r))) => {
                    unimplemented!();
                }
                Ok(Async::Ready(None)) => {
                    self.shutdown = true;
                    return Ok(self.shutdown());
                }
                Ok(Async::NotReady) => break,
            }
        }
        return Ok(Async::NotReady);
        */
    }
    fn poll_complete(&mut self) -> Result<Async<()>, Void> {
        unimplemented!()
    }
    fn close(&mut self) -> Result<Async<()>, Void> {
        unimplemented!();
    }
}
