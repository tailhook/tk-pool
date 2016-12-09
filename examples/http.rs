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
use std::sync::Arc;

use abstract_ns::Resolver;
use futures::Future;
use futures::stream::Stream;
use futures::future::join_all;
use futures::sink::Sink;
use futures::sync::mpsc::channel;
use tk_pool::uniform::{Pool, Config as PConfig};
use tokio_core::net::TcpStream;
use minihttp::client::{Proto, Config as HConfig, Client, Error};

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
    let pool_impl = Pool::new(&Arc::new(
            PConfig::new()
            .connections_per_address(2)
            .clone()
        ),
        &h1,
        ns.subscribe("httpbin.org:80"),
        move |addr| {
            Box::new(
                TcpStream::connect(&addr, &h2)
                .map(|c| Proto::new(c, &Arc::new(HConfig::new())))
                .map_err(Error::Io))
            as Box<Future<Item=_, Error=_>>
        });
    let (mut pool, rx) = channel(10);
    let fut = rx.map_err(|_| -> Error {
            unimplemented!();
        }).forward(pool_impl)
        .map(|_| println!("DONE"))
        .map_err(|e| println!("Error {:?}", e));
    lp.handle().spawn(fut);
    println!("We will send 10 requests over 2 connections (per ip). \
              Each requests hangs for 5 seconds at the server side \
              (a feature of httpbin.org). So expect script to finish \
              in 25 seconds with first response coming in 5 seconds.");
    lp.run(futures::lazy(|| {
        join_all((0..10).map(move |_| {
            let (codec, receiver) = minihttp::client::buffered::Buffered::
                get("http://httpbin.org/delay/5".parse().unwrap());
            assert!(pool.start_send(codec).unwrap().is_ready());
            receiver.map(|r| {
                println!("Received {} bytes", r.body().unwrap().len())
            })

            //pool.fetch_url(&mut pool, "http://httpbin.org/delay/5")
            //.map(|r| {
            //    println!("Received {} bytes", r.body().unwrap().len())
            //})
        }))
    })).unwrap();
}
