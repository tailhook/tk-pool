extern crate env_logger;
extern crate futures;
extern crate abstract_ns;
extern crate tokio_core;
extern crate tokio_redis as redis;
extern crate tokio_service;
extern crate ns_dns_tokio;
extern crate tk_pool;

use std::time::Duration;

use abstract_ns::Resolver;
use futures::Future;
use futures::stream::Stream;
use tokio_core::reactor::{Core, Interval};
use tokio_service::Service;
use redis::{Cmd, Client};
use ns_dns_tokio::DnsResolver;
use tk_pool::ConnectionPool;

pub fn main() {
    env_logger::init().unwrap();

    let mut lp = Core::new().unwrap();

    let resolver = DnsResolver::system_config(&lp.handle())
        .expect("initialize DNS resolver");
    //let mut resolver = abstract_ns::StubResolver::new();
    //resolver.add_host("localhost", "127.0.0.1".parse().unwrap());
    let handle = lp.handle();
    let handle2 = lp.handle();
    let handle3 = lp.handle();
    let pool = ConnectionPool::new(&lp.handle(),
        resolver.subscribe("localhost:6379"),
        move |addr| {
            println!("Hello {:?}", addr);
            Client::new().connect(&handle, &addr)
        });

    ;
    lp.run(Interval::new(Duration::new(1, 0), &handle2).unwrap()
        .for_each(|()| {
            let mut cmd = Cmd::new();
            cmd.arg("GET");
            cmd.arg("value");
            handle3.spawn(pool.call(cmd).then(|v| {
                println!("Value {:?}", v);
                Ok(())
            }));
            Ok(())
        })).unwrap();
}
