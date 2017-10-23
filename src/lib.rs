//! A connection pool implementation for tokio
//!
//! [Documentation](https://docs.rs/tk-pool) |
//! [Github](https://github.com/tailhook/tk-pool) |
//! [Crate](https://crates.io/crates/tk-pool)
//!
#[warn(missing_docs)]
#[warn(missing_debug_implementations)]
#[macro_use] extern crate log;
extern crate abstract_ns;
extern crate futures;
extern crate rand;
extern crate tokio_core;
extern crate void;

mod connect;
mod pool;

pub use connect::Connect;
pub use pool::Pool;
