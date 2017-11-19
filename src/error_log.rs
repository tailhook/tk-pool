use std::fmt;
use std::net::SocketAddr;
use std::marker::PhantomData;

use config::{NewErrorLog};

#[derive(Debug)]
pub enum ShutdownReason {
    RequestStreamClosed,
    AddressStreamClosed,
    #[doc(hidden)]
    __Nonexhaustive,
}

pub trait ErrorLog {
    type ConnectionError;
    type SinkError;
    fn connection_error(&self, _addr: SocketAddr, _e: Self::ConnectionError) {}
    fn sink_error(&self, _addr: SocketAddr, _e: Self::SinkError) {}
    fn pool_shutting_down(&self, _reason: ShutdownReason) {}
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
