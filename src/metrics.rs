/// An object implementing trait may collect metrics of a connection pool
pub trait Collect: Clone + Send + Sync {

}


/// A object implementing Collect which is not interested in actual metrics
#[derive(Debug, Clone)]
pub struct Noop;

impl Collect for Noop {}
