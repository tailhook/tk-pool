use std::rc::Rc;
use std::net::SocketAddr;
use std::cell::RefCell;
use std::collections::VecDeque;

use abstract_ns::Address;
use tokio_core::reactor::Handle;
use tokio_service::Service;
use futures::{Async, Future};
use futures::stream::Stream;


pub struct ConnectionPool<S: Service>(Rc<RefCell<PoolImpl<S>>>);

pub struct PoolImpl<S: Service> {
    connections: VecDeque<S>,
}

pub trait Connect {
    type Service;
    fn connect(&self, sock: SocketAddr) -> Self::Service;
}

impl<S: Service + 'static> ConnectionPool<S> {
    pub fn new<A, C>(handle: &Handle, addr: A, connect: C) -> ConnectionPool<S>
        where A: Stream<Item=Address> + 'static,
              C: Connect<Service=S> + 'static,
    {
        let pool = Rc::new(RefCell::new(PoolImpl {
            connections: VecDeque::new(),
        }));
        handle.spawn(addr
            .then(|res| Ok::<_, ()>(res))
            .fold((pool.clone(), connect),
                |(pool, connect), res| {
                    res.ok()
                        .and_then(|x| x.pick_one())
                        .map(|addr| {
                            pool.borrow_mut().connections
                                .push_back(connect.connect(addr))
                        });
                    Ok((pool, connect))
                })
            .then(|_| -> Result<_,_> {
                unreachable!();
            }));
        return ConnectionPool(pool);
    }
}

impl<S: Service, T:Fn(SocketAddr) -> S> Connect for T {
    type Service = S;
    fn connect(&self, sock: SocketAddr) -> Self::Service {
        self(sock)
    }
}

impl<S: Service> Service for ConnectionPool<S> {
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;
    fn call(&self, req: S::Request) -> S::Future {
        let conn = self.0.borrow_mut().connections.pop_front().unwrap();
        let result = conn.call(req);
        self.0.borrow_mut().connections.push_back(conn);
        return result;
    }
    fn poll_ready(&self) -> Async<()> {
        // Note: this is probably never called anyway
        let pool = self.0.borrow_mut();
        if pool.connections.len() == 0 {
            return Async::NotReady;
        }
        // TODO(tailhook) check poll_ready at connections in the pool
        return Async::Ready(());
    }
}

impl<S: Service> Clone for ConnectionPool<S> {
    fn clone(&self) -> ConnectionPool<S> {
        ConnectionPool(self.0.clone())
    }
}
