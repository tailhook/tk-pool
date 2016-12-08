use std::net::SocketAddr;

use futures::{Future};


/// This is a trait that is used for establishing a connection
///
/// Usually just passing a closure is good enough
pub trait Connect {
    type Sink;
    type Error;
    /// Establish a connection to the specified address
    fn connect(&mut self, address: SocketAddr)
        -> Box<Future<Item=Self::Sink, Error=Self::Error>>;
}

impl<S, E, T> Connect for T
    where T: FnMut(SocketAddr) -> Box<Future<Item=S, Error=E>>
{
    type Sink = S;
    type Error = E;

    fn connect(&mut self, address: SocketAddr)
        -> Box<Future<Item=S, Error=E>>
    {
        (self)(address)
    }
}
