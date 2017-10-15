extern crate tk_pool;
extern crate tk_http;
extern crate futures;
extern crate abstract_ns;
extern crate futures_cpupool;
extern crate tokio_core;
extern crate env_logger;
extern crate ns_std_threaded;
extern crate ns_router;

use std::env;

use futures::{Future, Sink};
use futures::future::join_all;
use tk_pool::uniform::{UniformMx, Config as PConfig};
use tk_pool::Pool;
use tk_http::client::{Proto, Config as HConfig, Client, Error};

fn main() {
    if let Err(_) = env::var("RUST_LOG") {
        env::set_var("RUST_LOG", "warn");
    }
    env_logger::init().unwrap();

    let mut lp = tokio_core::reactor::Core::new().unwrap();
    let h1 = lp.handle();
    let h2 = lp.handle();

    let ns = ns_router::Router::from_config(&ns_router::Config::new()
        .set_fallthrough_host_resolver(ns_std_threaded::ThreadedResolver::new())
        .done(),
        &lp.handle());
    let pool_config = PConfig::new()
        .connections_per_address(2)
        .done();
    let connection_config = HConfig::new()
        .inflight_request_limit(1)
        .done();
    let multiplexer = UniformMx::new(&h1,
        &pool_config,
        ns.subscribe_many(&["httpbin.org:80"], 80),
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
