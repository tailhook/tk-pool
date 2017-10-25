use std::time::Duration;

use futures::{Future, Stream, Async};
use tokio_core::reactor::Handle;

use config::{ErrorLog, NewMux};
use connect::Connect;
use metrics::Collect;
use void::{Void, unreachable};


/// A constructor for a uniform connection pool with lazy connections
pub struct LazyUniform {
    pub(crate) conn_limit: usize,
    pub(crate) reconnect_timeout: Duration,
}

struct Lazy<S, C, E, M> {
    conn_limit: usize,
    reconnect_ms: (u64, u64),  // easier to make random value
    shutdown: bool,
    requests: S,
    connector: C,
    errors: E,
    metrics: M,
}

impl NewMux for LazyUniform {
    fn spawn_on<S, C, E, M>(self, h: &Handle, requests: S,
        connector: C, errors: E, metrics: M)
        where C: Connect + 'static,
              E: ErrorLog<ConnectionError=<C::Future as Future>::Error>,
              E: 'static,
              M: Collect + 'static,
              S: Stream<Error=Void> + 'static,
    {
        let reconn_ms = self.reconnect_timeout.as_secs() * 1000 +
            (self.reconnect_timeout.subsec_nanos() / 1000_000) as u64;
        h.spawn(Lazy {
            conn_limit: self.conn_limit,
            reconnect_ms: (reconn_ms / 2, reconn_ms * 3 / 2),
            shutdown: false,
            requests, connector, errors, metrics,
        });
    }
}

impl<S, C, E, M> Lazy<S, C, E, M>
    where C: Connect,
          S: Stream<Error=Void>,
          E: ErrorLog<ConnectionError=<C::Future as Future>::Error>,
          M: Collect,
{
    fn shutdown(&mut self) -> Async<()> {
        unimplemented!();
    }

    fn process_requests(&mut self) {
        unimplemented!();
    }
}

impl<S, C, E, M> Future for Lazy<S, C, E, M>
    where C: Connect,
          S: Stream<Error=Void>,
          E: ErrorLog<ConnectionError=<C::Future as Future>::Error>,
          M: Collect,
{
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Result<Async<()>, ()> {
        if self.shutdown {
            return Ok(self.shutdown());
        }
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
    }
}
