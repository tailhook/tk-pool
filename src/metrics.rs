/// An object implementing trait may collect metrics of a connection pool
pub trait Collect: Clone + Send + Sync {
    fn connection_attempt(&self) {}
    /// Connection established
    fn connection(&self) {}
    fn connection_error(&self) {}
    fn disconnect(&self) {}
}


/// A object implementing Collect which is not interested in actual metrics
#[derive(Debug, Clone)]
pub struct Noop;

impl Collect for Noop {}
