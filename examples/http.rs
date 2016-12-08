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
use futures::sink::Sink;
use tk_pool::uniform::{Pool, Config as PConfig};
use tokio_core::net::TcpStream;
use minihttp::client::{Proto, Config as HConfig, Client, Error};

fn main() {
    let mut lp = tokio_core::reactor::Core::new().unwrap();
    let h1 = lp.handle();
    let h2 = lp.handle();

    let ns = ns_std_threaded::ThreadedResolver::new(
        futures_cpupool::CpuPool::new(1));
    let mut pool = Pool::new(&Arc::new(PConfig::new()),
        &h1,
        ns.subscribe("httpbin.org:80"),
        move |addr| {
            Box::new(
                TcpStream::connect(&addr, &h2)
                .map(|c| Proto::new(c, &Arc::new(HConfig::new())))
                .map_err(Error::Io))
            as Box<Future<Item=_, Error=_>>
        }).buffer(10);
    lp.run(futures::lazy(|| {
        join_all((0..10).map(move |_| {
            Client::fetch_url(&mut pool, "http://httpbin.org/delay/5")
            .map(|r| {
                println!("Received {} bytes", r.body().unwrap().len())
            })
        }))
    })).unwrap();
}
