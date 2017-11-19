use std::net::SocketAddr;

use futures::{Future, IntoFuture};


/// This is a trait that is used for establishing a connection
///
/// Usually just passing a closure is good enough
pub trait Connect {
    /// A future retuned by `connect` method
    type Future: Future;
    /// Establish a connection to the specified address
    fn connect(&mut self, address: SocketAddr) -> Self::Future;
}

impl<T, F> Connect for T
    where T: FnMut(SocketAddr) -> F,
          F: IntoFuture,
{
    type Future = <T::Output as IntoFuture>::Future;
    fn connect(&mut self, address: SocketAddr) -> Self::Future {
        (self)(address).into_future()
    }
}
