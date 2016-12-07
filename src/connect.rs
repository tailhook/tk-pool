use std::net::SocketAddr;

use futures::Future;


/// This is a trait that is used for establishing a connection
///
/// Usually just passing a closure is good enough
pub trait Connect {
    type Future: Future;
    /// Establish a connection to the specified address
    fn connect(&mut self, address: SocketAddr) -> Self::Future;
}

impl<S, E, T, F> Connect for T
    where T: FnMut(SocketAddr) -> F, F: Future<Item=S, Error=E>
{
    type Future = F;
    fn connect(&mut self, address: SocketAddr) -> Self::Future {
        (self)(address)
    }
}

