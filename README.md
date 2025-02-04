# Fastimer

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![MSRV 1.83][msrv-badge]](https://www.whatrustisit.com)
[![Apache 2.0 licensed][license-badge]][license-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/fastimer.svg
[crates-url]: https://crates.io/crates/fastimer
[docs-badge]: https://docs.rs/fastimer/badge.svg
[msrv-badge]: https://img.shields.io/badge/MSRV-1.83-green?logo=rust
[docs-url]: https://docs.rs/fastimer
[license-badge]: https://img.shields.io/crates/l/fastimer
[license-url]: LICENSE
[actions-badge]: https://github.com/fast/fastimer/workflows/CI/badge.svg
[actions-url]:https://github.com/fast/fastimer/actions?query=workflow%3ACI

## Overview

Fastimer implements runtime-agnostic timer traits and utilities.

### Scheduled Actions

Fastimer provides scheduled actions that can be scheduled as a repeating and cancellable action.

* `SimpleAction`: A simple repeatable action that can be scheduled with a fixed delay, or at a fixed rate.
* `ArbitraryDelayAction`: A repeatable action that can be scheduled with arbitrary delay.
* `NotifyAction`: A repeatable action that can be scheduled by notifications.

### Timeout

* `Timeout` is a future combinator that completes when the inner future completes or when the timeout expires.

### Interval

* `Interval` ticks at a sequence of instants with a certain duration between each instant.

## Installation

Add the dependency to your `Cargo.toml` via:

```shell
cargo add fastimer
```

## Documentation

Read the online documents at https://docs.rs/fastimer.

## Minimum Supported Rust Version (MSRV)

This crate is built against the latest stable release, and its minimum supported rustc version is 1.80.0.

The policy is that the minimum Rust version required to use this crate can be increased in minor version updates. For example, if Fastimer 1.0 requires Rust 1.20.0, then Fastimer 1.0.z for all values of z will also require Rust 1.20.0 or newer. However, Fastimer 1.y for y > 0 may require a newer minimum version of Rust.

## License

This project is licensed under [Apache License, Version 2.0](LICENSE).
