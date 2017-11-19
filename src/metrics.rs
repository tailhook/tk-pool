/// An object implementing trait may collect metrics of a connection pool
pub trait Collect: Clone + Send + Sync {
    fn connection_attempt(&self) {}
    /// Aborted connection attempt (name changed)
    fn connection_abort(&self) {}
    /// Connection established
    fn connection(&self) {}
    fn connection_error(&self) {}
    fn disconnect(&self) {}

    fn blacklist_add(&self) {}
    fn blacklist_remove(&self) {}

    fn request_queued(&self) {}
    fn request_forwarded(&self) {}

    fn pool_closed(&self) {}
}


/// A object implementing Collect which is not interested in actual metrics
#[derive(Debug, Clone)]
pub struct Noop;

impl Collect for Noop {}
