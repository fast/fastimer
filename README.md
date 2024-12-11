# Fastimer

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![MSRV 1.75][msrv-badge]](https://www.whatrustisit.com)
[![Apache 2.0 licensed][license-badge]][license-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/fastimer.svg
[crates-url]: https://crates.io/crates/fastimer
[docs-badge]: https://docs.rs/fastimer/badge.svg
[msrv-badge]: https://img.shields.io/badge/MSRV-1.75-green?logo=rust
[docs-url]: https://docs.rs/fastimer
[license-badge]: https://img.shields.io/crates/l/fastimer
[license-url]: LICENSE
[actions-badge]: https://github.com/fast/fastimer/workflows/CI/badge.svg
[actions-url]:https://github.com/fast/fastimer/actions?query=workflow%3ACI

## Overview

Fastimer implements runtime-agnostic driver for async timers and scheduled tasks.

### Scheduled Actions

Fastimer provides [`SimpleAction`](https://docs.rs/fastimer/latest/fastimer/schedule/trait.SimpleAction.html) and [`ArbitraryDelayAction`](https://docs.rs/fastimer/latest/fastimer/schedule/trait.ArbitraryDelayAction.html) that can be scheduled as a repeating and cancellable action.

### Timeout

[`Timeout`](https://docs.rs/fastimer/latest/fastimer/struct.Timeout.html) is a future combinator that completes when the inner future completes or when the timeout expires.

### Time Driver

[`TimeDriver`](https://docs.rs/fastimer/latest/fastimer/driver/struct.TimeDriver.html) is a runtime-agnostic time driver for creating delay futures. To use the time driver, you need to enable the driver feature flag.

## Installation

Add the dependency to your `Cargo.toml` via:

```shell
cargo add mea
```

## Documentation

Read the online documents at https://docs.rs/fastimer.

## License

This project is licensed under [Apache License, Version 2.0](LICENSE).
