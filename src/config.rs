use abstract_ns::Address;
use futures::{Future, Stream, Sink};
use tokio_core::reactor::Handle;
use void::Void;

use connect::Connect;
use pool::Pool;

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
pub struct Queue(usize);

/// A constructor for a default error logger
pub struct WarnLogger;

/// A constructor for a default (no-op) metrics collector
pub struct NoopMetrics;

/// A constructor for a uniform connection pool with lazy connections
pub struct LazyUniform(usize);

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
        -> Pool<<<<C as Connect>::Future as Future>::Item as Sink>::SinkItem>
        where C: Connect,
              <<C as Connect>::Future as Future>::Item: Sink,
    {
        unimplemented!();
    }

    /// Configure a uniform connection pool with specified number of
    /// connections crated lazily (i.e. when there are requests)
    pub fn lazy_uniform_connections(self, num: usize)
        -> PoolConfig<C, A, LazyUniform, Q, E, M>
    {
        PoolConfig {
            mux: LazyUniform(num),
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


