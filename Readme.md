coinched
========

A [coinche](https://en.wikipedia.org/wiki/Coinche) server.

It uses [libcoinche](https://github.com/Gyscos/libcoinche) to model a game of
coinche, and presents it as a network service, for example as a HTTP interface.

To run the default HTTP API: 

```
cargo run
```

Note: `coinched` currently requires rust>=1.6, which won't be stable until
January 20. Until then, use rust beta or nightly.
