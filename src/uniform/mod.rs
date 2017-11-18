mod aligner;
mod chan;
mod connect;
mod failures;
mod sink;

use std::cell::RefCell;
use std::collections::{VecDeque, HashSet};
use std::net::SocketAddr;
use std::rc::Rc;
use std::time::{Duration, Instant};

use abstract_ns::Address;
use futures::{Future, Async, Sink, AsyncSink, Stream};
use futures::stream::FuturesUnordered;
use rand::{thread_rng, Rng};
use tokio_core::reactor::Handle;
use void::{Void, unreachable};

use config::{ErrorLog, NewMux};
use connect::Connect;
use metrics::Collect;
use queue::Done;
use uniform::aligner::Aligner;
use uniform::chan::{Controller, Helper};
use uniform::connect::ConnectFuture;
use uniform::failures::Blacklist;
use uniform::sink::SinkFuture;


pub enum FutureOk<S> {
    Connected(SocketAddr, S),
}

pub enum FutureErr<E, F> {
    CantConnect(SocketAddr, E),
    Disconnected(SocketAddr, F),
}

/// A constructor for a uniform connection pool with lazy connections
pub struct LazyUniform {
    pub(crate) conn_limit: u32,
    pub(crate) reconnect_timeout: Duration,
}

pub struct Connections<I> {
    queue: VecDeque<Controller<I>>,
    all: HashSet<Controller<I>>,
}

pub struct Lazy<A, C, E, M>
    where E: ErrorLog,
          C: Connect,
          <<C as Connect>::Future as Future>::Item: Sink,
{
    conn_limit: u32,
    reconnect_ms: (u64, u64),  // easier to make random value
    futures: FuturesUnordered<Box<Future<
        Item=FutureOk<<C::Future as Future>::Item>,
        Error=FutureErr<E::ConnectionError, E::SinkError>>>>,
    connections: Rc<RefCell<Connections<
        <<<C as Connect>::Future as Future>::Item as Sink>::SinkItem>>>,
    address: A,
    connector: C,
    errors: E,
    metrics: M,
    aligner: Aligner,
    blist: Blacklist,
    cur_address: Address,
    shutdown: bool,
}

impl<I> Connections<I> {
    fn new() -> Connections<I>{
        Connections {
            queue: VecDeque::new(),
            all: HashSet::new(),
        }
    }
    fn add(&mut self, ctr: Controller<I>) {
        {
            let mut inner = ctr.inner.borrow_mut();
            assert!(!inner.closed);
            assert!(!inner.queued);
            inner.queued = true;
        }
        self.queue.push_back(ctr);
    }
    fn next(&mut self) -> Option<Controller<I>> {
        self.queue.pop_front()
        .map(|ctr| {
            {
                let mut inner = ctr.inner.borrow_mut();
                assert!(inner.queued);
                inner.queued = false;
            }
            ctr
        })
    }
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
            connections: Rc::new(RefCell::new(Connections::new())),
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
          <<C as Connect>::Future as Future>::Item: Sink,
          E: ErrorLog<
            ConnectionError=<C::Future as Future>::Error,
            SinkError=<<C::Future as Future>::Item as Sink>::SinkError,
          >,
          M: Collect + 'static,
{
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
        self.cur_address = new_addr;
        // TODO(tailhook) retire old connections
    }
    fn do_connect(&mut self) -> Option<SocketAddr> {
        let ref blist = self.blist;
        let new = self.aligner.get(self.conn_limit, |a| blist.is_failing(a));
        if let Some(addr) = new {
            self.metrics.connection_attempt();
            self.futures.push(
                Box::new(ConnectFuture::new(addr,
                    self.connector.connect(addr))));
            debug!("Connecting to {}", addr);
            return Some(addr);
        }
        return None;
    }
    fn poll_futures(&mut self) {
        loop {
            match self.futures.poll() {
                Ok(Async::NotReady) => break,
                Ok(Async::Ready(Some(FutureOk::Connected(addr, sink)))) => {
                    self.metrics.connection();
                    let task = Helper::new(addr, self.connections.clone());
                    self.connections.borrow_mut()
                        .all.insert(task.controller());
                    // helper will add itself to the active queue on wakeup
                    self.futures.push(Box::new(SinkFuture::new(sink, task)));
                }
                Err(FutureErr::CantConnect(sa, err)) => {
                    self.metrics.connection_error();
                    self.errors.connection_error(sa, err);
                    let (min, max) = self.reconnect_ms;
                    let dur = Duration::from_millis(
                            thread_rng().gen_range(min, max));
                    self.metrics.blacklist_add();
                    self.blist.blacklist(sa, Instant::now() + dur);
                    self.aligner.put(sa);
                }
                Err(FutureErr::Disconnected(sa, err)) => {
                    self.metrics.disconnect();
                    // TODO(tailhook) blacklist connection if it was
                    // recently connected
                    self.errors.sink_error(sa, err);
                    self.aligner.put(sa);
                }
                // we just put another future and it did not exit
                // yet
                Ok(Async::Ready(None)) => unreachable!(),
            }
        }
    }
}

impl<A, C, E, M> Sink for Lazy<A, C, E, M>
    where A: Stream<Item=Address, Error=Void>,
          C: Connect + 'static,
          <C::Future as Future>::Item: Sink,
          E: ErrorLog<
            ConnectionError=<C::Future as Future>::Error,
            SinkError=<<C::Future as Future>::Item as Sink>::SinkError>,
          M: Collect + 'static,
{
    type SinkItem = <<C::Future as Future>::Item as Sink>::SinkItem;
    type SinkError = Done;
    fn start_send(&mut self, mut v: Self::SinkItem)
        -> Result<AsyncSink<Self::SinkItem>, Done>
    {
        if self.shutdown {
            self.poll_futures();
            if self.futures.len() == 0 {
                return Err(Done);
            }
            return Ok(AsyncSink::NotReady(v));
        } else {
            self.check_for_address_updates();
            loop {
                let ctr = self.connections.borrow_mut().next();
                if let Some(ctr) = ctr {
                    if ctr.is_closed() { continue }
                    ctr.request(v);
                    self.poll_futures();
                    if let Some(request) = ctr.request_back() {
                        v = request;
                        continue;
                    } else {
                        // Note: we assume that controller put itself back
                        // to the active queue
                        return Ok(AsyncSink::Ready);
                    }
                } else {
                    break;
                }
            }
            loop {
                while let Some(addr) = self.do_connect() {
                    self.poll_futures();
                    if !self.blist.is_failing(addr) {
                        // Waiting for connect
                        return Ok(AsyncSink::NotReady(v));
                    }
                }
                if let Async::Ready(_) = self.blist.poll() {
                    self.metrics.blacklist_remove();
                } else {
                    // log backpressure issue, not sure how
                    return Ok(AsyncSink::NotReady(v));
                }
            }
        }
    }
    fn poll_complete(&mut self) -> Result<Async<()>, Done> {
        if self.shutdown {
            self.poll_futures();
            if self.futures.len() == 0 {
                return Err(Done);
            }
            return Ok(Async::NotReady);
        } else {
            self.poll_futures();
        }
        // TODO(tailhook) maybe we can track if connections have everything
        // flushed
        return Ok(Async::NotReady);
    }
    fn close(&mut self) -> Result<Async<()>, Done> {
        self.shutdown = true;
        // TODO(tailhook) close underlying sinks
        self.poll_futures();
        if self.futures.len() == 0 {
            return Ok(Async::Ready(()));
        }
        return Ok(Async::NotReady);
    }
}

