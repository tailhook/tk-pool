//! This is a collection of different connection pool implementations
//!
//! Note the API is still in flux.
//!
//! All connection pools work on a `futures::sink::Sink`. When you use this
//! module for request-reply style options you should have something like
//! `Sink<(Request, Sender<Reply>)>`.
//!
#[warn(missing_docs)]
#[macro_use] extern crate log;
extern crate futures;
extern crate rand;
extern crate abstract_ns;
extern crate tokio_core;

mod connect;
pub mod uniform;

pub use connect::Connect;
