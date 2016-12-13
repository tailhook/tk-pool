//! This module provides and uniform connection pool implementation,
//! which means we create a fixed number of connections for each IP/Port pair
//! and distribute requests by round-robin
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::VecDeque;

use rand::{thread_rng, Rng};
use abstract_ns::{self, Address};
use tokio_core::reactor::Handle;
use futures::{StartSend, AsyncSink, Async, Future, Poll};
use futures::sink::{Sink};
use futures::stream::Stream;

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
pub struct UniformMx<S, E, A=abstract_ns::Error> {
    address: Box<Stream<Item=Address, Error=A>>,
    connect: Box<Connect<Sink=S, Error=E>>,
    cur_address: Option<Address>,
    active: VecDeque<(SocketAddr, S)>,
    pending: VecDeque<(SocketAddr, Box<Future<Item=S, Error=E>>)>,
    candidates: Vec<SocketAddr>,
    retired: VecDeque<S>,
    config: Arc<Config>,
    handle: Handle,
}


/// Configuration of the connection pool
///
/// Note default configuration doesn't make sense for most cases. Please
/// tweak at least `Config::connections_per_address`.
///
/// Also make sure to use `eager_connections()` if you expect performance.
#[derive(Clone, Debug)]
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
    /// sense if you have `HashMap<Hostname, UniformMx>` and this is how most
    /// connections pools work in other languages.
    ///
    /// Note that pool with lazy connections will return NotReady when there
    /// are free connections, but starts a new ones asynchronously. Also it
    /// will not establish new connections when there are no backpressure on
    /// existing connections even if not all peers are connected to yet. So you
    /// may get a skew in cluster load, especially if you support may
    /// pipelined requests on a single connection.
    pub fn eager_connections(&mut self) -> &mut Self {
        self.lazy_connections = false;
        self
    }
    /// Set the number of connections per address
    ///
    /// This kind of limit may look awkward for a connection pool. You used
    /// to opening fixed number of connections per connection pool rather
    /// than per backend address. But consider there are lots of workers
    /// (say 100 or 1000), and every worker has limited number of resources.
    /// So you want to have a number of connections proportional to actual
    /// workers there rather than arbitrarily chosen number of connections.
    ///
    /// Surely it doesn't work well for other cases. This is why we have
    /// multiple multiplexer implementations.
    pub fn connections_per_address(&mut self, n: usize) -> &mut Self {
        self.connections_per_address = n;
        self
    }
    /// Create a Arc'd config clone to pass to the constructor
    ///
    /// This is just a convenience method.
    pub fn done(&mut self) -> Arc<Config> {
        Arc::new(self.clone())
    }
}

impl<S, E, A> UniformMx<S, E, A>
    where S: Sink<SinkError=E>,
          E: From<A> + fmt::Display
{
    /// Create a connection pool
    ///
    /// This doesn't establish any connections even in eager mode. You need
    /// to call `poll_complete` to start.
    pub fn new<C>(handle: &Handle, config: &Arc<Config>,
           address: Box<Stream<Item=Address, Error=A>>, connect: C)
        -> UniformMx<S, E, A>
        where C: Connect<Sink=S, Error=E> + 'static
    {
        UniformMx {
            address: address,
            connect: Box::new(connect),
            active: VecDeque::new(),
            pending: VecDeque::new(),
            config: config.clone(),
            handle: handle.clone(),
            cur_address: None,
            candidates: Vec::new(),
            retired: VecDeque::new(),
        }
    }

    fn try_send(&mut self, mut item: S::SinkItem)
        -> Result<AsyncSink<S::SinkItem>, E>
    {
        for _ in 0..self.active.len() {
            let (a, mut c) = self.active.pop_front().unwrap();
            item = match c.start_send(item)? {
                AsyncSink::NotReady(item) => {
                    self.active.push_back((a, c));
                    item
                }
                AsyncSink::Ready => {
                    self.active.push_back((a, c));
                    return Ok(AsyncSink::Ready);
                }
            };
        }
        Ok(AsyncSink::NotReady(item))
    }
    /// Returns `true` if new connection became active
    fn do_poll(&mut self) -> Result<(), E> {
        match self.address.poll() {
            Ok(Async::Ready(Some(new_addr))) => {
                if let Some(ref mut old_addr) = self.cur_address {
                    if old_addr != &new_addr {
                        // Retire connections immediately, but connect later
                        let (old, new) = old_addr.at(0)
                                       .compare_addresses(&new_addr.at(0));
                        debug!("New addresss, to be retired {:?}, \
                                to be connected {:?}", old, new);
                        for _ in 0..self.pending.len() {
                            let (addr, c) = self.pending.pop_front().unwrap();
                            // Drop pending connections to non-existing
                            // addresses
                            if !old.contains(&addr) {
                                self.pending.push_back((addr, c));
                            } else {
                                debug!("Dropped pending {}", addr);
                            }
                        }
                        for _ in 0..self.active.len() {
                            let (addr, c) = self.active.pop_front().unwrap();
                            // Active connections are waiting to become idle
                            if old.contains(&addr) {
                                debug!("Retiring {}", addr);
                                self.retired.push_back(c);
                            } else {
                                self.active.push_back((addr, c));
                            }
                        }
                        // New addresses go to the front of the list but
                        // we randomize their order
                        for _ in 0..self.config.connections_per_address {
                            let off = self.candidates.len();
                            for &a in &new {
                                self.candidates.push(a);
                            }
                            thread_rng().shuffle(&mut self.candidates[off..]);
                        }
                        *old_addr = new_addr;
                    }
                } else {
                    // We randomize order of the connections, but make sure
                    // that no connection connects twice unless all other are
                    // connected at least once
                    for _ in 0..self.config.connections_per_address {
                        let off = self.candidates.len();
                        for a in new_addr.at(0).addresses() {
                            self.candidates.push(a);
                        }
                        thread_rng().shuffle(&mut self.candidates[off..]);
                    }
                    self.cur_address = Some(new_addr);
                }
            },
            Ok(Async::NotReady) => {}
            Ok(Async::Ready(None)) => {
                panic!("Address stream must be infinite");
            }
            Err(e) => {
                // TODO(tailhook) poll crashes on address error?
                return Err(e.into());
            }
        }
        for _ in 0..self.retired.len() {
            let mut c = self.retired.pop_front().unwrap();
            match c.poll_complete() {
                Ok(Async::Ready(())) => {}
                Ok(Async::NotReady) => {
                    self.retired.push_back(c);
                }
                Err(_) => {}  // TODO(tailhook) may be log crashes?
            }
        }
        for _ in 0..self.active.len() {
            let (a, mut c) = self.active.pop_front().unwrap();
            match c.poll_complete() {
                Ok(_) => self.active.push_back((a, c)),
                Err(e) => {
                    info!("Connection to {:?} has been closed: {}", a, e);
                    // Add to the end of the list
                    self.candidates.insert(0, a);
                }
            }
        }
        self.poll_pending();
        Ok(())
    }

    fn poll_pending(&mut self) -> bool {
        let mut added = false;
        for _ in 0..self.pending.len() {
            let (a, mut c) = self.pending.pop_front().unwrap();
            match c.poll() {
                Ok(Async::Ready(c)) => {
                    // Can use it immediately
                    debug!("Connected {}", a);
                    self.active.push_front((a, c));
                    added = true;
                }
                Ok(Async::NotReady) => {
                    self.pending.push_back((a, c));
                }
                Err(e) => {
                    info!("Can't establish connection to {:?}: {}", a, e);
                    // Add to the end of the list
                    self.candidates.insert(0, a);
                }
            }
        }
        added
    }
    /// Initiate a connection(s)
    fn do_connect(&mut self) {
        // TODO(tailhook) implement some timeouts for failing connections
        if self.config.lazy_connections {
            if let Some(addr) = self.candidates.pop() {
                debug!("Connecting to {} (lazy mode)", addr);
                self.pending.push_back((addr, self.connect.connect(addr)));
                // we need to poll here, basicallly to schedule wakeup
                self.poll_pending();
            }
        } else {
            while let Some(addr) = self.candidates.pop() {
                debug!("Connecting to {}", addr);
                self.pending.push_back((addr, self.connect.connect(addr)));
            }
            // we need to poll here, basicallly to schedule wakeups
            self.poll_pending();
        }
        self.candidates.shrink_to_fit();
    }
}


impl<S, E, A> Sink for UniformMx<S, E, A>
    where S: Sink<SinkError=E>,
          E: From<A> + fmt::Display,
{
    type SinkItem = S::SinkItem;
    type SinkError = S::SinkError;
    fn start_send(&mut self, item: Self::SinkItem)
        -> StartSend<Self::SinkItem, Self::SinkError>
    {
        self.do_poll()?;
        let item = match self.try_send(item)? {
            AsyncSink::NotReady(item) => item,
            AsyncSink::Ready => return Ok(AsyncSink::Ready),
        };
        self.do_connect();
        Ok(AsyncSink::NotReady(item))
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError>
    {
        if !self.config.lazy_connections {
            self.do_connect();
        }
        self.do_poll()?;
        // Basically we're never ready
        Ok(Async::NotReady)
    }
}
