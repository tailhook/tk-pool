//! This is a collection of different connection pool implementations
//!
//! # Concepts
//!
//! ## Mutliplexer
//!
//! Multiplexer is basically a strategy of how we establish connections,
//! distribute the load, reconnect in case of failure and process name service
//! updates.
//!
//! For now we only have one multiplexer `uniform::UniformMx`. It estblishes
//! fixed number of connections to each address that name service returned.
//!
//! Technically multiplexer is a `Sink` that creates and manages other
//! sinks.
//!
//! Use multiplexer directly if you want to build other abstraction on top
//! of it: for example to build a `HashMap<Name, Multiplexer>`. For user code,
//! multiplexer should be used via `Pool`.
//!
//!
//! ## Pool
//!
//! Pool is an object that is convenient to use for client connections. It
//! will spawn a future that processes requests and establish a channel to it.
//! It can be used from multiple threads (but will establish connections in
//! an the one that originally created pool).
//!
//! It's still a `Sink` and you are free to implement `Service` or whatever
//! high level interfaces apply for your protocol.
//!
//! If you need pool of connections having **request-reply** you create a
//! sink of `SinkItem=(Request, Receiver<Reply>)`.
//!
//! Good example of usage is `minihttp::client::Client` trait which implements
//! simple methods like `fetch_url` on the `Sink`. So the method applies
//! both to connection pools and to individual connections.
//!
//! # Example
//!
//! ```rust,ignore
//! let pool_config = PConfig::new()
//!     .connections_per_address(2)
//!     .done();
//! let multiplexer = UniformMx::new(
//!     &h1,
//!     &pool_config,
//!     ns.subscribe("example.org:80"),
//!     move |addr| Proto::connect_tcp(addr, &connection_config, &h2));
//! let queue_size = 10;
//! let mut pool = Pool::create(&lp.handle(), queue_size, multiplexer);
//! ```
//!
//! # Notes
//!
//! Note the API is still in flux.
//!
//!
#[warn(missing_docs)]
#[macro_use] extern crate log;
extern crate futures;
extern crate rand;
extern crate abstract_ns;
extern crate tokio_core;

mod connect;
mod pool;
pub mod uniform;

pub use connect::Connect;
pub use pool::Pool;
