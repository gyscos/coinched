coinched
========

[![Build Status](https://travis-ci.org/gyscos/coinched.svg?branch=master)](https://travis-ci.org/gyscos/coinched)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

A [coinche](https://en.wikipedia.org/wiki/Coinche) server.

It uses [libcoinche](https://github.com/Gyscos/libcoinche) to model a game of
coinche, and presents it as a network service, for example as a HTTP interface.

To run the default HTTP server:

```
cargo run --bin coinched -- --port 3000
```

To run the proof-of-concept HTTP client:

```
cargo run --bin coincher -- localhost:3000
```
