//! A number of traits to configure connection pool and their implementatons
//!
//! Usually you should start with ``pool_for`` and use methods to configure
//! connection pool instead of poking at these types.
//!
use std::time::Duration;

use abstract_ns::Address;
use futures::{Future, Stream, Sink};
use tokio_core::reactor::Handle;
use void::Void;

use error_log::{ErrorLog, WarnLogger};
use connect::Connect;
use metrics::{self, Collect};
use uniform::LazyUniform;

/// A constructor for metrics collector object used for connection pool
pub trait NewMetrics {
    type Collect: Collect;
    fn construct(self) -> Self::Collect;
}

/// A constructor for queue
///
/// This trait is currently *sealed*, we will unseal it once it stabilized
pub trait NewQueue<I, M>: private::NewQueue<I, M> {
    /// Connection pool instance type
    type Pool;
}

impl<I, M, T: private::NewQueue<I, M>> NewQueue<I, M> for T {
    type Pool = T::Pool;
}

/// A constructor for multiplexer
///
/// This trait is currently *sealed*, we will unseal it once it stabilized
pub trait NewMux<A, C, E, M>: private::NewMux<A, C, E, M>
    where A: Stream<Item=Address, Error=Void>,
          C: Connect + 'static,
          <<C as Connect>::Future as Future>::Item: Sink,
          E: ErrorLog<
            ConnectionError=<C::Future as Future>::Error,
            SinkError=<<C::Future as Future>::Item as Sink>::SinkError,
            >,
          E: 'static,
          M: Collect + 'static,
{}

pub(crate) mod private {
    use futures::{Stream, Future, Sink};
    use void::Void;
    use connect::Connect;
    use metrics::Collect;
    use error_log::ErrorLog;
    use abstract_ns::Address;
    use tokio_core::reactor::Handle;

    pub struct Done;

    pub trait NewMux<A, C, E, M>
        where A: Stream<Item=Address, Error=Void>,
              C: Connect + 'static,
              <<C as Connect>::Future as Future>::Item: Sink,
              E: ErrorLog<
                ConnectionError=<C::Future as Future>::Error,
                SinkError=<<C::Future as Future>::Item as Sink>::SinkError,
                >,
              E: 'static,
              M: Collect + 'static,
    {
        type Sink: Sink<
            SinkItem=<<C::Future as Future>::Item as Sink>::SinkItem,
            SinkError=Done,
        >;
        fn construct(self,
            h: &Handle, address: A, connector: C, errors: E, metrics: M)
            -> Self::Sink;
    }

    pub trait NewQueue<I, M> {
        type Pool;
        fn spawn_on<S, E>(self, pool: S, e: E, metrics: M, handle: &Handle)
            -> Self::Pool
            where S: Sink<SinkItem=I, SinkError=Done> + 'static,
                  E: ErrorLog + 'static,
                  M: Collect + 'static;
    }

}

/// A constructor for error log
pub trait NewErrorLog<C, S> {
    type ErrorLog: ErrorLog<ConnectionError=C, SinkError=S>;
    fn construct(self) -> Self::ErrorLog;
}

/// A configuration builder that holds onto `Connect` object
#[derive(Debug)]
pub struct PartialConfig<C> {
    pub(crate) connector: C,
}

/// A fully configured pool but you might override some defaults
pub struct PoolConfig<C, A, X, Q, E, M> {
    pub(crate) connector: C,
    pub(crate) address: A,
    pub(crate) mux: X,
    pub(crate) queue: Q,
    pub(crate) errors: E,
    pub(crate) metrics: M,
}

/// A constructor for a default multiplexer
pub struct DefaultMux;

/// A constructor for a default queue
pub struct DefaultQueue;

/// A constructor for a fixed-size dumb queue
pub struct Queue(pub(crate) usize);

/// A constructor for a default (no-op) metrics collector
pub struct NoopMetrics;

impl NewMetrics for NoopMetrics {
    type Collect = metrics::Noop;
    fn construct(self) -> metrics::Noop {
        metrics::Noop
    }
}

impl<C> PartialConfig<C> {
    /// Create a configuration by adding an address stream
    pub fn connect_to<A>(self, address_stream: A)
        -> PoolConfig<C, A, DefaultMux, DefaultQueue, WarnLogger, NoopMetrics>
        where A: Stream<Item=Address, Error=Void>,
    {
        PoolConfig {
            address: address_stream,
            connector: self.connector,
            mux: DefaultMux,
            errors: WarnLogger,
            queue: DefaultQueue,
            metrics: NoopMetrics,
        }
    }
}

impl<C, A, X, Q, E, M> PoolConfig<C, A, X, Q, E, M> {
    /// Spawn a connection pool on the main loop specified by handle
    pub fn spawn_on(self, h: &Handle)
        -> <Q as NewQueue<
                <<<C as Connect>::Future as Future>::Item as Sink>::SinkItem,
                <M as NewMetrics>::Collect,
           >>::Pool
        where A: Stream<Item=Address, Error=Void>,
              C: Connect + 'static,
              <<C as Connect>::Future as Future>::Item: Sink,
              M: NewMetrics,
              M::Collect: 'static,
              X: NewMux<A, C, E::ErrorLog, M::Collect>,
              <X as private::NewMux<A, C, E::ErrorLog, M::Collect>>::Sink: 'static,
              E: NewErrorLog<
                <<C as Connect>::Future as Future>::Error,
                <<<C as Connect>::Future as Future>::Item as Sink>::SinkError,
              >,
              E::ErrorLog: Clone + 'static,
              Q: NewQueue<
                <<<C as Connect>::Future as Future>::Item as Sink>::SinkItem,
                <M as NewMetrics>::Collect,
                Pool=<Q as private::NewQueue<
                    <<<C as Connect>::Future as Future>::Item as Sink>::SinkItem,
                    <M as NewMetrics>::Collect,
                >>::Pool
              >,

    {
        let m = self.metrics.construct();
        let e = self.errors.construct();
        let p = self.mux.construct(h,
            self.address, self.connector, e.clone(), m.clone());
        self.queue.spawn_on(p, e, m, h)
    }

    /// Configure a uniform connection pool with specified number of
    /// per-host connections crated lazily (i.e. when there are requests)
    pub fn lazy_uniform_connections(self, num: u32)
        -> PoolConfig<C, A, LazyUniform, Q, E, M>
    {
        PoolConfig {
            mux: LazyUniform {
                conn_limit: num,
                reconnect_timeout: Duration::from_millis(100),
            },
            address: self.address,
            connector: self.connector,
            errors: self.errors,
            queue: self.queue,
            metrics: self.metrics,
        }
    }

    /// Add a queue of size num used when no connection can accept a message
    pub fn with_queue_size(self, num: usize)
        -> PoolConfig<C, A, X, Queue, E, M>
    {
        PoolConfig {
            queue: Queue(num),
            address: self.address,
            connector: self.connector,
            mux: self.mux,
            errors: self.errors,
            metrics: self.metrics,
        }
    }

    /// Override metrics reporter
    pub fn metrics<NM>(self, metrics: NM)
        -> PoolConfig<C, A, X, Q, E, NM>
        where NM: NewMetrics,
    {
        PoolConfig {
            queue: self.queue,
            address: self.address,
            connector: self.connector,
            mux: self.mux,
            errors: self.errors,
            metrics: metrics,
        }
    }

    /// Override error reporter
    pub fn errors<NE>(self, errors: NE)
        -> PoolConfig<C, A, X, Q, NE, M>
        where NE: ErrorLog,
    {
        PoolConfig {
            queue: self.queue,
            address: self.address,
            connector: self.connector,
            mux: self.mux,
            errors: errors,
            metrics: self.metrics,
        }
    }
}


