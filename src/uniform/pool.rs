use std::cell::RefCell;
use std::rc::Rc;

use abstract_ns::Address;
use futures::{Future, Sink};
use futures::stream::FuturesUnordered;

use error_log::{ErrorLog};
use connect::Connect;
use uniform::aligner::Aligner;
use uniform::failures::Blacklist;
use uniform::{Connections, FutureOk, FutureErr};


pub struct Lazy<A, C, E, M>
    where E: ErrorLog,
          C: Connect,
          <<C as Connect>::Future as Future>::Item: Sink,
{
    pub(in uniform) conn_limit: u32,
    pub(in uniform) reconnect_ms: (u64, u64),  // easier to make random value
    pub(in uniform) futures: FuturesUnordered<Box<Future<
                        Item=FutureOk<<C::Future as Future>::Item>,
                        Error=FutureErr<E::ConnectionError, E::SinkError>>>>,
    pub(in uniform) connections: Rc<RefCell<Connections<
                        <<<C as Connect>::Future as Future>::Item as Sink>::SinkItem>>>,
    pub(in uniform) address: A,
    pub(in uniform) connector: C,
    pub(in uniform) errors: E,
    pub(in uniform) metrics: M,
    pub(in uniform) aligner: Aligner,
    pub(in uniform) blist: Blacklist,
    pub(in uniform) cur_address: Address,
    pub(in uniform) closing: bool,
}
