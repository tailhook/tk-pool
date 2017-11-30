extern crate tk_pool;
extern crate tk_http;
extern crate futures;
extern crate abstract_ns;
extern crate futures_cpupool;
extern crate tokio_core;
extern crate env_logger;
extern crate ns_std_threaded;
extern crate ns_router;
extern crate log;

use std::env;
use std::time::Duration;

use abstract_ns::HostResolve;
use futures::{Future, Sink};
use futures::future::join_all;
use tk_pool::{pool_for};
use tk_http::client::{Proto, Config as HConfig, Client, Error};
use ns_router::SubscribeExt;

fn main() {
    if let Err(_) = env::var("RUST_LOG") {
        env::set_var("RUST_LOG", "warn");
    }
    env_logger::init().unwrap();

    let mut lp = tokio_core::reactor::Core::new().unwrap();
    let h1 = lp.handle();
    let h2 = lp.handle();

    let ns = ns_router::Router::from_config(&ns_router::Config::new()
        .set_fallthrough(ns_std_threaded::ThreadedResolver::new()
            .null_service_resolver()
            .interval_subscriber(Duration::new(1, 0), &h1))
        .done(),
        &lp.handle());

    let connection_config = HConfig::new()
        .inflight_request_limit(1)
        .done();
    let mut pool =
        pool_for(move |addr| Proto::connect_tcp(addr, &connection_config, &h2))
        .connect_to(ns.subscribe_many(&["httpbin.org"], 80))
        .lazy_uniform_connections(2)
        .with_queue_size(16)
        .spawn_on(&lp.handle())
        // This is needed for Client trait, (i.e. so that `.fetch_url()` works)
        // May be fixed in tk-http in future
        .sink_map_err(|_| Error::custom("Can't send request"));

    // Alternative config (not implemented)
    // let mut pool =
    //     pool_for(|addr| Proto::connect_tcp(addr, &connection_config, &h2))
    //     .with_config_stream(ns, once_and_wait(Config::new()
    //         .set_name(&["httpbin.org:80"])
    //         .lazy_uniform_connections(2)
    //         .with_queue_size(10)))
    //     .spawn_on(&lp.handle());

    println!("We will send 16 requests over 1 connection per ip. \
              Each requests hangs for 5 seconds at the server side \
              (a feature of httpbin.org). Since httpbin nowadays has \
              many IPs, expect script to finish in 5 or 10 seconds \
              with first response coming in 5 seconds.");

    lp.run(futures::lazy(|| {
        join_all((0..16).map(move |_| {
            pool
            .fetch_url("http://httpbin.org/delay/5")
            .map(|r| {
                println!("Received {} bytes", r.body().len())
            })
        }))
    })).unwrap();
}
