extern crate tk_pool;
extern crate tk_http;
extern crate futures;
extern crate abstract_ns;
extern crate futures_cpupool;
extern crate tokio_core;
extern crate env_logger;
extern crate ns_std_threaded;
extern crate log;

use std::env;

use abstract_ns::Resolver;
use futures::{Future, Stream};
use futures::future::join_all;
use tk_pool::uniform::{UniformMx, Config as PConfig};
use tk_pool::Pool;
use tk_http::client::{Proto, Config as HConfig, Client, Error};

use sink_map_err::SinkExt;

fn main() {
    if let Err(_) = env::var("RUST_LOG") {
        env::set_var("RUST_LOG", "warn");
    }
    env_logger::init().unwrap();

    let mut lp = tokio_core::reactor::Core::new().unwrap();
    let h1 = lp.handle();
    let h2 = lp.handle();

    let ns = ns_std_threaded::ThreadedResolver::new(
        futures_cpupool::CpuPool::new(1));
    let pool_config = PConfig::new()
        .connections_per_address(2)
        .done();
    let connection_config = HConfig::new()
        .inflight_request_limit(1)
        .done();
    let multiplexer = UniformMx::new(&h1,
        &pool_config,
        ns.subscribe("httpbin.org:80").map_err(|e| Error::custom(e)),
        move |addr| Proto::connect_tcp(addr, &connection_config, &h2));
    let queue_size = 10;
    let mut pool = Pool::create(&lp.handle(), queue_size, multiplexer)
            .sink_map_err(|_| Error::custom("Can't send request"));

    println!("We will send 10 requests over 2 connections (per ip). \
              Each requests hangs for 5 seconds at the server side \
              (a feature of httpbin.org). So expect script to finish \
              in 25 seconds with first response coming in 5 seconds.");

    lp.run(futures::lazy(|| {
        join_all((0..10).map(move |_| {
            pool
            .fetch_url("http://httpbin.org/delay/5")
            .map(|r| {
                println!("Received {} bytes", r.body().len())
            })
        }))
    })).unwrap();
}

// till next release of `futures`
mod sink_map_err {

    use futures::{Poll, StartSend};
    use futures::sink::Sink;

    pub trait SinkExt: Sink {
        /// Transforms the error returned by the sink.
        fn sink_map_err<F, E>(self, f: F) -> SinkMapErr<Self, F>
            where F: FnOnce(Self::SinkError) -> E,
                  Self: Sized,
        {
            new(self, f)
        }
    }

    impl<T: Sink> SinkExt for T {}

    /// Sink for the `Sink::sink_map_err` combinator.
    #[derive(Debug)]
    #[must_use = "sinks do nothing unless polled"]
    pub struct SinkMapErr<S, F> {
        sink: S,
        f: Option<F>,
    }

    pub fn new<S, F>(s: S, f: F) -> SinkMapErr<S, F> {
        SinkMapErr { sink: s, f: Some(f) }
    }

    impl<S, F, E> Sink for SinkMapErr<S, F>
        where S: Sink,
              F: FnOnce(S::SinkError) -> E,
    {
        type SinkItem = S::SinkItem;
        type SinkError = E;

        fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
            self.sink.start_send(item).map_err(|e| self.f.take().expect("cannot use MapErr after an error")(e))
        }

        fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
            self.sink.poll_complete().map_err(|e| self.f.take().expect("cannot use MapErr after an error")(e))
        }
    }
}
