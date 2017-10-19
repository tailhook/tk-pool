Tk-Pool
=======

**Status: Alpha**

[Documentation](https://docs.rs/tk-pool) |
[Github](https://github.com/tailhook/tk-pool) |
[Crate](https://crates.io/crates/tk-pool) |
[Examples](https://github.com/tailhook/tk-pool/tree/master/examples)


A connection pool implementation for tokio. Main features:

1. Works for any request-reply protocol (actually for any `Sink`)
2. Provides both queue and pushback if needed
3. Allows pipelining (multiple in-flight request when multiplexing)
4. Auto-reconnects on broken connection
5. Adapts when DNS name change
6. Can have both eagerly and lazily established connections

Multiple load-balancing strategies are in to do list.


License
=======

Licensed under either of

* Apache License, Version 2.0,
  (./LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license (./LICENSE-MIT or http://opensource.org/licenses/MIT)
  at your option.

Contribution
------------

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

