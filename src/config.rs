use std::fmt;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::time::Duration;

use abstract_ns::Address;
use futures::{Future, Stream, Sink};
use tokio_core::reactor::Handle;
use void::Void;

use connect::Connect;
use metrics::{self, Collect};
use uniform::LazyUniform;

/// A constructor for metrics collector object used for connection pool
pub trait NewMetrics {
    type Collect: Collect;
    fn construct(self) -> Self::Collect;
}

/// A constructor for queue
pub trait NewQueue<I, M> {
    type Pool;
    fn spawn_on<S>(self, pool: S, metrics: M, handle: &Handle) -> Self::Pool
        where S: Sink<SinkItem=I, SinkError=Void>;
}

/// A constructor for multiplexer
pub trait NewMux<C, E, M>
    where C: Connect + 'static,
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
        SinkError=Void,
    >;
    fn construct(self,
        h: &Handle, connector: C, errors: E, metrics: M)
        -> Self::Sink;
}

pub trait NewErrorLog<C, S> {
    type ErrorLog: ErrorLog<ConnectionError=C, SinkError=S>;
    fn construct(self) -> Self::ErrorLog;
}

pub trait ErrorLog {
    type ConnectionError;
    type SinkError;
    fn connection_error(self, addr: SocketAddr, e: Self::ConnectionError);
    fn sink_error(self, addr: SocketAddr, e: Self::SinkError);
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

/// A constructor for a default error logger
pub struct WarnLogger;

/// An instance of default error logger
pub struct WarnLoggerInstance<C, S>(PhantomData<* const (C, S)>);

/// A constructor for a default (no-op) metrics collector
pub struct NoopMetrics;

impl NewMetrics for NoopMetrics {
    type Collect = metrics::Noop;
    fn construct(self) -> metrics::Noop {
        metrics::Noop
    }
}

impl<C: fmt::Display, S: fmt::Display> NewErrorLog<C, S> for WarnLogger {
    type ErrorLog = WarnLoggerInstance<C, S>;
    fn construct(self) -> Self::ErrorLog {
        WarnLoggerInstance(PhantomData)
    }
}

impl<C, S> ErrorLog for WarnLoggerInstance<C, S>
    where C: fmt::Display,
          S: fmt::Display,
{
    type ConnectionError = C;
    type SinkError = S;
    fn connection_error(self, addr: SocketAddr, e: Self::ConnectionError) {
        warn!("Connecting to {} failed: {}", addr, e);
    }
    fn sink_error(self, addr: SocketAddr, e: Self::SinkError) {
        warn!("Connection to {} errored: {}", addr, e);
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
        where C: Connect + 'static,
              <<C as Connect>::Future as Future>::Item: Sink,
              M: NewMetrics,
              M::Collect: 'static,
              X: NewMux<C, E::ErrorLog, M::Collect>,
              E: NewErrorLog<
                <<C as Connect>::Future as Future>::Error,
                <<<C as Connect>::Future as Future>::Item as Sink>::SinkError,
              >,
              E::ErrorLog: 'static,
              Q: NewQueue<
                <<<C as Connect>::Future as Future>::Item as Sink>::SinkItem,
                <M as NewMetrics>::Collect,
              >,

    {
        let m = self.metrics.construct();
        let e = self.errors.construct();
        let p = self.mux.construct(h, self.connector, e, m.clone());
        self.queue.spawn_on(p, m, h)
    }

    /// Configure a uniform connection pool with specified number of
    /// per-host connections crated lazily (i.e. when there are requests)
    pub fn lazy_uniform_connections(self, num: usize)
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
}


