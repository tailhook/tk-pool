mod failures;
mod aligner;
mod connect;

use std::time::Duration;
use std::net::SocketAddr;

use abstract_ns::Address;
use futures::{Future, Async, Sink, AsyncSink, Stream};
use futures::stream::FuturesUnordered;
use tokio_core::reactor::Handle;

use config::{ErrorLog, NewMux};
use connect::Connect;
use metrics::Collect;
use void::{Void, unreachable};
use uniform::aligner::Aligner;
use uniform::failures::Blacklist;
use uniform::connect::ConnectFuture;
use queue::Done;


pub enum FutureOk {
}

pub enum FutureErr {
}

/// A constructor for a uniform connection pool with lazy connections
pub struct LazyUniform {
    pub(crate) conn_limit: u32,
    pub(crate) reconnect_timeout: Duration,
}

pub struct Lazy<A, C, E, M> {
    conn_limit: u32,
    reconnect_ms: (u64, u64),  // easier to make random value
    futures: FuturesUnordered<Box<Future<Item=FutureOk, Error=FutureErr>>>,
    address: A,
    connector: C,
    errors: E,
    metrics: M,
    aligner: Aligner,
    blist: Blacklist,
    cur_address: Address,
    shutdown: bool,
}

impl<A, C, E, M> NewMux<A, C, E, M> for LazyUniform
    where A: Stream<Item=Address, Error=Void>,
          C: Connect + 'static,
          <<C as Connect>::Future as Future>::Item: Sink,
          E: ErrorLog<
            ConnectionError=<C::Future as Future>::Error,
            SinkError=<<C::Future as Future>::Item as Sink>::SinkError,
            >,
          E: 'static,
          M: Collect + 'static,
{
    type Sink = Lazy<A, C, E, M>;
    fn construct(self,
        h: &Handle, address: A, connector: C, errors: E, metrics: M)
        -> Lazy<A, C, E, M>
    {
        let reconn_ms = self.reconnect_timeout.as_secs() * 1000 +
            (self.reconnect_timeout.subsec_nanos() / 1000_000) as u64;
        Lazy {
            conn_limit: self.conn_limit,
            reconnect_ms: (reconn_ms / 2, reconn_ms * 3 / 2),
            futures: FuturesUnordered::new(),
            blist: Blacklist::new(h),
            aligner: Aligner::new(),
            shutdown: false,
            cur_address: [][..].into(),
            address, connector, errors, metrics,
        }
    }
}

impl<A, C, E, M> Lazy<A, C, E, M>
    where A: Stream<Item=Address, Error=Void>,
          C: Connect + 'static,
          E: ErrorLog<ConnectionError=<C::Future as Future>::Error>,
          M: Collect,
{

    fn process_requests(&mut self) {
        unimplemented!();
    }
    fn new_addr(&mut self) -> Option<Address> {
        let mut result = None;
        loop {
            match self.address.poll() {
                Ok(Async::Ready(Some(addr))) => result = Some(addr),
                Ok(Async::Ready(None)) => {
                    self.shutdown = true;
                    result = None;
                    break;
                }
                Ok(Async::NotReady) => break,
                Err(e) => unreachable(e),
            }
        }
        return result;
    }
    fn check_for_address_updates(&mut self) {
        let new_addr = match self.new_addr() {
            Some(new) => {
                if new != self.cur_address {
                    new
                } else {
                    return;
                }
            }
            _ => return,
        };
        let (old, new) = self.cur_address.at(0)
                       .compare_addresses(&new_addr.at(0));
        debug!("New address, to be retired {:?}, \
                to be connected {:?}", old, new);
        self.aligner.update(new, old);
        // TODO(tailhook) retire old connections
    }
    fn do_connect(&mut self) -> Option<SocketAddr> {
        let ref blist = self.blist;
        let new = self.aligner.get(self.conn_limit, |a| blist.is_failing(a));
        if let Some(addr) = new {
            self.futures.push(
                Box::new(ConnectFuture::new(self.connector.connect(addr))));
            return Some(addr);
        }
        return None;
    }
    fn poll_shutdown(&mut self) -> bool {
        unimplemented!();
        //match self.futures.poll() {
        //    Ok(_) => {}
        //    Err(_) => {}
        //}
    }
}

impl<A, C, E, M> Sink for Lazy<A, C, E, M>
    where A: Stream<Item=Address, Error=Void>,
          C: Connect + 'static,
          <C::Future as Future>::Item: Sink,
          E: ErrorLog<
            ConnectionError=<C::Future as Future>::Error,
            SinkError=<<C::Future as Future>::Item as Sink>::SinkError>,
          M: Collect,
{
    type SinkItem = <<C::Future as Future>::Item as Sink>::SinkItem;
    type SinkError = Done;
    fn start_send(&mut self, v: Self::SinkItem)
        -> Result<AsyncSink<Self::SinkItem>, Done>
    {
        if self.shutdown {
            if self.poll_shutdown() {
                return Err(Done);
            }
            return Ok(AsyncSink::NotReady(v));
        } else {
            self.check_for_address_updates();
            loop {
                if let Some(addr) = self.do_connect() {
                    unimplemented!();
                    // match self.futures.poll() {
                    // }
                    // if addr matches failure, do connect again
                } else {
                    return Ok(AsyncSink::NotReady(v));
                }
            }
        }
    }
    fn poll_complete(&mut self) -> Result<Async<()>, Done> {
        unimplemented!();
        // match self.futures.poll() {
        // }
    }
    fn close(&mut self) -> Result<Async<()>, Done> {
        unimplemented!();
    }
}
