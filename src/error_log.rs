use std::fmt;
use std::net::SocketAddr;
use std::marker::PhantomData;

use config::{NewErrorLog, ErrorLog};

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
    fn connection_pool_shut_down(&self) {
        warn!("Connection pool shut down");
    }
}
