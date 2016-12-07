//! This module provides and uniform connection pool implementation,
//! which means we create a fixed number of connections for each IP/Port pair
//! and distribute requests by round-robin
use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::VecDeque;

use abstract_ns::{self, Address};
use tokio_core::reactor::Handle;
use futures::{StartSend, Async, Future, BoxFuture, Poll};
use futures::sink::{Sink};
use futures::stream::{Stream, BoxStream};

use {Connect};


/// A simple uniform connection pool
///
/// This pool connects fixed number of connections
/// to each IP/Port pair (if they are available) and distribute requests
/// by round-robin
///
/// Note the pool has neither a buffer of it's own nor any internal tasks, so
/// you are expected to use `Sink::buffer` and call `poll_complete` on every
/// wake-up.
pub struct Pool<S, E, A=abstract_ns::Error> {
    address: BoxStream<Address, A>,
    active: VecDeque<S>,
    pending: VecDeque<BoxFuture<S, E>>,
    config: Arc<Config>,
    handle: Handle,
}


/// Configuration of the connection pool
///
/// Note default configuration doesn't make sense for most cases. Please
/// tweak at least `Config::connections_per_address`.
///
/// Also make sure to use `eager_connections()` if you expect performance.
pub struct Config {
    connections_per_address: usize,
    lazy_connections: bool,
}

impl Config {
    /// Create a new config with default configuration
    ///
    /// Default configuration has `connections_per_address: 1` which is only
    /// useful if you have synchronous workers and only one of them is bound
    /// to every socket, or if there is a virtually infinite resources on the
    /// other side (comparing to the number of requests we are going to do)
    /// and good pipelining support.
    pub fn new() -> Config {
        Config {
            connections_per_address: 1,
            lazy_connections: true,
        }
    }

    /// Establish connections and keep them open even if there are no requests.
    ///
    /// Lazy connections are nicer when you have mostly idle connection pool
    /// and don't need sub-millisecond latency. The connection is established
    /// only when there are no other connections that can serve a request.
    ///
    /// Lazy connections are enabled by default because it what makes most
    /// sense if you have `HashMap<Hostname, Pool>` and this is how most
    /// connections pools work in other languages.
    pub fn eager_connections(&mut self) -> &mut Self {
        self.lazy_connections = false;
        self
    }
}

impl<S, E, A> Pool<S, E, A>
    where S: Sink<SinkError=E>,
          A: Into<E>
{
    /// Create a connection pool
    ///
    /// This doesn't establish any connections even in eager mode. You need
    /// to call `poll_complete` to start.
    pub fn new<C>(config: &Arc<Config>, handle: &Handle,
           address: BoxStream<Address, A>, connect: C)
        -> Pool<S, E, A>
        where C: Connect,
              C::Future: Future<Item=S, Error=E>,
    {
        Pool {
            address: address,
            active: VecDeque::new(),
            pending: VecDeque::new(),
            config: config.clone(),
            handle: handle.clone(),
        }
    }
}

impl<S, E, A> Sink for Pool<S, E, A>
    where S: Sink<SinkError=E>,
          A: Into<E>,
{
    type SinkItem = S::SinkItem;
    type SinkError = S::SinkError;
    fn start_send(&mut self, item: Self::SinkItem)
        -> StartSend<Self::SinkItem, Self::SinkError>
    {
        unimplemented!();
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError>
    {
        unimplemented!();
    }
}
