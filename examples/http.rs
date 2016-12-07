extern crate tk_pool;
extern crate minihttp;
extern crate futures;
extern crate abstract_ns;
extern crate futures_cpupool;
extern crate tokio_core;
extern crate ns_std_threaded;

use std::sync::Arc;

use abstract_ns::Resolver;
use futures::Future;
use futures::future::join_all;
use tk_pool::uniform::{Pool, Config as PConfig};
use tokio_core::net::TcpStream;
use minihttp::client::{Proto, Config as HConfig, Client};

fn main() {
    let lp = tokio_core::reactor::Core::new().unwrap();
    let handle = lp.handle();

    let ns = ns_std_threaded::ThreadedResolver::new(
        futures_cpupool::CpuPool::new(1));
    let pool = Pool::new(&Arc::new(PConfig::new()),
        &handle,
        ns.subscribe("httpbin.org"),
        |addr| {
            TcpStream::connect(&addr, &handle)
            .map(|c| Proto::new(c, &Arc::new(HConfig::new())))
        });
    lp.run(join_all((0..10).map(|_| {
        Client::fetch_url(&mut pool, "http://httpbin.org/delay/5")
            .map(|r| {
                println!("Received {} bytes", r.body().unwrap().len())
            })
    }))).unwrap();
}
