//! A connection pool implementation for tokio
//!
//! [Documentation](https://docs.rs/tk-pool) |
//! [Github](https://github.com/tailhook/tk-pool) |
//! [Crate](https://crates.io/crates/tk-pool) |
//! [Examples](https://github.com/tailhook/tk-pool/tree/master/examples)
//!
//! A connection pool implementation for tokio. Main features:
//!
//! 1. Works for any request-reply protocol (actually for any `Sink`)
//! 2. Provides both queue and pushback if needed
//! 3. Allows pipelining (multiple in-flight request when multiplexing)
//! 4. Auto-reconnects on broken connection
//! 5. Adapts when DNS name change
//!
//! Multiple load-balancing strategies are in to do list.
//!
//! # Example
//!
//! ```rust,ignore
//!
//! let mut pool =
//!     pool_for(|addr| connect(addr))
//!     .connect_to(ns.subscribe_many(address, default_port))
//!     .lazy_uniform_connections(2)
//!     .with_queue_size(10)
//!     .spawn_on(&core.handle());
//!
//! ```
//!
#[macro_use] extern crate log;
extern crate abstract_ns;
extern crate futures;
extern crate rand;
extern crate tokio_core;
extern crate void;

mod connect;
mod basic;
pub mod queue;
pub mod error_log;
pub mod metrics;
pub mod uniform;
pub mod config;

pub use basic::pool_for;
pub use connect::Connect;
