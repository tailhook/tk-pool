//! Metrics trait and no-op implementation

/// An object implementing trait may collect metrics of a connection pool
pub trait Collect: Clone + Send + Sync {
    /// We started establishing connection
    ///
    /// This pairs either with ``connection`` on success
    /// or ``connection_abort`` and ``connection_error`` on error
    fn connection_attempt(&self) {}
    /// Error establishing connection
    fn connection_error(&self) {}

    /// Aborted connection attempt (name changed, and doesn't include address)
    fn connection_abort(&self) {}
    /// Connection established successfully
    fn connection(&self) {}
    /// Connection closed (usully means name changed)
    fn disconnect(&self) {}

    /// Host address added to a blacklist (i.e. connection error)
    fn blacklist_add(&self) {}
    /// Host address removed from a blacklist
    ///
    /// Note this callback is only called when we're searching for a new
    /// connection and all others are busy. I.e. unlisting a host from a
    /// blacklist may be delayed arbitrarily when not under backpressure.
    /// This may be fixed in future.
    fn blacklist_remove(&self) {}

    /// Request queued in the internal queue
    fn request_queued(&self) {}
    /// Request unqueued from the internal queue and forwarded to a sink
    ///
    /// Note: this might not mean that request is already sent as we can't
    /// control the underlying sinks used.
    fn request_forwarded(&self) {}

    /// Connection pool is closed
    fn pool_closed(&self) {}
}


/// A object implementing Collect which is not interested in actual metrics
#[derive(Debug, Clone)]
pub struct Noop;

impl Collect for Noop {}
