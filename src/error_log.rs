//! ErrorLog trait and default implementations
//!
use std::fmt;
use std::net::SocketAddr;
use std::marker::PhantomData;

use config::{NewErrorLog};

/// A reason connection pool being shut down
///
/// This value is passed to ``ErrorLog::pool_shutting_down`` method.
#[derive(Debug)]
pub enum ShutdownReason {
    /// Request stream is shutting down
    ///
    /// This usually means that all clones of ``queue::Pool`` object are
    /// destroyed
    RequestStreamClosed,
    /// Address stream is closed
    ///
    /// Shutting down address stream is commonly used to force shut down
    /// connection pool, even if there are users. Users will get an error
    /// on the next `start_send`.
    AddressStreamClosed,
    #[doc(hidden)]
    __Nonexhaustive,
}


/// The thrait that has appropriate hooks for logging different events
///
/// There is a default ``WarnLogger``, but the idea is that you may give
/// connection pool a name, change logging levels, and do other interesting
/// stuff in your own error log handler.
pub trait ErrorLog {
    /// Connection error type that is returned by connect/hanshake function
    type ConnectionError;
    /// Error when sending request returned by Sink
    type SinkError;
    /// Error when establishing a new connection
    fn connection_error(&self, _addr: SocketAddr, _e: Self::ConnectionError) {}
    /// Error when sending a request
    ///
    /// This also means connection is closed
    fn sink_error(&self, _addr: SocketAddr, _e: Self::SinkError) {}
    /// Pool is started to shut down for the specified reason
    fn pool_shutting_down(&self, _reason: ShutdownReason) {}
    /// Pool is fully closed at this moment
    fn pool_closed(&self) {}
}


/// A constructor for a default error logger
pub struct WarnLogger;

/// An instance of default error logger
pub struct WarnLoggerInstance<C, S>(PhantomData<* const (C, S)>);

impl<C, S> Clone for WarnLoggerInstance<C, S> {
    fn clone(&self) -> Self {
        WarnLoggerInstance(PhantomData)
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
    fn connection_error(&self, addr: SocketAddr, e: Self::ConnectionError) {
        warn!("Connecting to {} failed: {}", addr, e);
    }
    fn sink_error(&self, addr: SocketAddr, e: Self::SinkError) {
        warn!("Connection to {} errored: {}", addr, e);
    }
    /// Starting to shut down pool
    fn pool_shutting_down(&self, reason: ShutdownReason) {
        warn!("Shutting down connection pool: {}", reason);
    }
    /// This is triggered when pool done all the work and shut down entirely
    fn pool_closed(&self) {
    }
}

impl fmt::Display for ShutdownReason {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::ShutdownReason::*;
        f.write_str(match *self {
            RequestStreamClosed => "request stream closed",
            AddressStreamClosed => "address stream closed",
            __Nonexhaustive => unreachable!(),
        })
    }
}
