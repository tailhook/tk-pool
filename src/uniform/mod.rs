mod failures;
mod aligner;

use std::time::Duration;

use abstract_ns::Address;
use futures::{Future, Async, Sink, AsyncSink, Stream};
use tokio_core::reactor::Handle;

use config::{ErrorLog, NewMux};
use connect::Connect;
use metrics::Collect;
use void::{Void, unreachable};
use uniform::aligner::Aligner;
use uniform::failures::Blacklist;


/// A constructor for a uniform connection pool with lazy connections
pub struct LazyUniform {
    pub(crate) conn_limit: u32,
    pub(crate) reconnect_timeout: Duration,
}

pub struct Lazy<A, C, E, M> {
    conn_limit: u32,
    reconnect_ms: (u64, u64),  // easier to make random value
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
          C: Connect,
          E: ErrorLog<ConnectionError=<C::Future as Future>::Error>,
          M: Collect,
{

    fn process_requests(&mut self) {
        unimplemented!();
    }
    fn do_connect(&mut self) {
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
}

impl<A, C, E, M> Sink for Lazy<A, C, E, M>
    where A: Stream<Item=Address, Error=Void>,
          C: Connect,
          <C::Future as Future>::Item: Sink,
          E: ErrorLog<
            ConnectionError=<C::Future as Future>::Error,
            SinkError=<<C::Future as Future>::Item as Sink>::SinkError>,
          M: Collect,
{
    type SinkItem = <<C::Future as Future>::Item as Sink>::SinkItem;
    type SinkError = Void;
    fn start_send(&mut self, v: Self::SinkItem)
        -> Result<AsyncSink<Self::SinkItem>, Void>
    {
        self.check_for_address_updates();
        unimplemented!()
        /*
        loop {
            self.process_requests();
            // TODO(tailhook) if has capacity
            match self.requests.poll() {
                Err(e) => unreachable(e),
                Ok(Async::Ready(Some(r))) => {
                    unimplemented!();
                }
                Ok(Async::Ready(None)) => {
                    self.shutdown = true;
                    return Ok(self.shutdown());
                }
                Ok(Async::NotReady) => break,
            }
        }
        return Ok(Async::NotReady);
        */
    }
    fn poll_complete(&mut self) -> Result<Async<()>, Void> {
        unimplemented!()
    }
    fn close(&mut self) -> Result<Async<()>, Void> {
        unimplemented!();
    }
}
