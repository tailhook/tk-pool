use std::time::Duration;

use futures::Future;
use tokio_core::reactor::Handle;

use config::{ErrorLog, NewMux};
use connect::Connect;
use metrics::Collect;

/// A constructor for a uniform connection pool with lazy connections
pub struct LazyUniform {
    pub(crate) size: usize,
    pub(crate) reconnect_timeout: Duration,
}

impl NewMux for LazyUniform {
    fn spawn_on<S, C, E, M>(self, h: &Handle, requests: S,
        connector: C, errors: E, metrics: M)
        where C: Connect,
              E: ErrorLog<ConnectionError=<C::Future as Future>::Error>,
              M: Collect
    {
        unimplemented!();
    }
}
