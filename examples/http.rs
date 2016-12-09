extern crate tk_pool;
extern crate minihttp;
extern crate futures;
extern crate abstract_ns;
extern crate futures_cpupool;
extern crate tokio_core;
extern crate env_logger;
extern crate ns_std_threaded;
#[macro_use] extern crate log;

use std::env;

use abstract_ns::Resolver;
use futures::Future;
use futures::future::join_all;
use tk_pool::uniform::{UniformMx, Config as PConfig};
use tk_pool::Pool;
use minihttp::client::{Proto, Config as HConfig, Client};

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
    let mut pool = Pool::create(&lp.handle(), 10, UniformMx::new(&h1,
            &pool_config,
            ns.subscribe("httpbin.org:80"),
            move |addr| Proto::connect_tcp(addr, &connection_config, &h2)));

    println!("We will send 10 requests over 2 connections (per ip). \
              Each requests hangs for 5 seconds at the server side \
              (a feature of httpbin.org). So expect script to finish \
              in 25 seconds with first response coming in 5 seconds.");

    lp.run(futures::lazy(|| {
        join_all((0..10).map(move |_| {
            pool.fetch_url("http://httpbin.org/delay/5")
            .map(|r| {
                println!("Received {} bytes", r.body().unwrap().len())
            })
        }))
    })).unwrap();
}
