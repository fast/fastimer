# Fastimer

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![MSRV 1.85][msrv-badge]](https://www.whatrustisit.com)
[![Apache 2.0 licensed][license-badge]][license-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/fastimer.svg
[crates-url]: https://crates.io/crates/fastimer
[docs-badge]: https://docs.rs/fastimer/badge.svg
[msrv-badge]: https://img.shields.io/badge/MSRV-1.85-green?logo=rust
[docs-url]: https://docs.rs/fastimer
[license-badge]: https://img.shields.io/crates/l/fastimer
[license-url]: LICENSE
[actions-badge]: https://github.com/fast/fastimer/workflows/CI/badge.svg
[actions-url]:https://github.com/fast/fastimer/actions?query=workflow%3ACI

> [!WARNING]
> The code for this project could serve as a starting point for runtime-agnostic timers and scheduled actions. However, I'm not entirely satisfied with the API or the name.
>
> Regarding the scheduled actions, I don't think they need to be treated specially. Essentially, everything you put in a `spawn.spawn`, you could just return a future and let an external spawn drive it.
>
> There are some things I want to change, but I haven't fully thought through its shape, its name, or where it should be placed. So, I'll temporarily put this project on hold.
>
> Its implementation is solid, though. If you want to reuse something like this for runtime-agnostic intervals, timeouts, or scheduled actions, the code is perfectly fine. I just feel there's significant room for improvement in its form and categorization, and I haven't quite figured it out yet, so I'll leave it as is for now.

For code sharing, below is what ScopeDB currently replace `fastimer` with:

```rust
use std::time::Duration;
use std::time::Instant;

use mea::shutdown::ShutdownRecv;

/// Create a far future instant.
pub fn far_future() -> Instant {
    // Roughly 30 years from now.
    // API does not provide a way to obtain max `Instant`
    // or convert specific date in the future to instant.
    // 1000 years overflows on macOS, 100 years overflows on FreeBSD.
    Instant::now() + Duration::from_secs(86400 * 365 * 30)
}

/// Create an instant from the given instant and a duration.
pub fn make_instant_from(now: Instant, dur: Duration) -> Instant {
    now.checked_add(dur).unwrap_or_else(far_future)
}

/// Create an instant from [`Instant::now`] and a duration.
pub fn make_instant_from_now(dur: Duration) -> Instant {
    make_instant_from(Instant::now(), dur)
}

#[derive(Debug, PartialEq, Eq)]
pub struct Elapsed(());

pub struct Interval(tokio::time::Interval);

impl Interval {
    pub async fn tick(&mut self) -> Instant {
        self.0.tick().await.into_std()
    }
}

pub trait TimerExt {
    type Output;

    fn timeout(self, duration: Duration) -> impl Future<Output = Result<Self::Output, Elapsed>>;

    fn timeout_at(self, at: Instant) -> impl Future<Output = Result<Self::Output, Elapsed>>;
}

impl<T, F: Future<Output = T>> TimerExt for F {
    type Output = T;

    async fn timeout(self, duration: Duration) -> Result<T, Elapsed> {
        tokio::time::timeout(duration, self)
            .await
            .map_err(|_| Elapsed(()))
    }

    async fn timeout_at(self, at: Instant) -> Result<Self::Output, Elapsed> {
        tokio::time::timeout_at(tokio::time::Instant::from_std(at), self)
            .await
            .map_err(|_| Elapsed(()))
    }
}

#[derive(Debug)]
pub struct Timer(());

pub fn timer() -> &'static Timer {
    static TIMER: Timer = Timer(());
    &TIMER
}

impl Timer {
    pub async fn delay_until(&self, at: Instant) {
        tokio::time::sleep_until(tokio::time::Instant::from_std(at)).await;
    }

    pub async fn delay(&self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }

    pub fn interval(&self, period: Duration) -> Interval {
        Interval(tokio::time::interval(period))
    }
}

impl Timer {
    /// Creates and executes a periodic task scheduled with fixed delay.
    ///
    /// The task will execute the first time after the given `initial_delay`, and subsequently
    /// with the given `delay` between the termination of one execution and the commencement of
    /// the next.
    pub async fn schedule_with_fixed_delay<F: AsyncFnMut()>(
        &self,
        name: &'static str,
        mut task: F,
        shutdown_rx: ShutdownRecv,
        initial_delay: Option<Duration>,
        delay: Duration,
    ) {
        log::info!(
            "start scheduled task {name} with fixed delay {delay:?} and initial delay {initial_delay:?}",
        );

        if let Some(initial_delay) = initial_delay
            && initial_delay > Duration::ZERO
        {
            tokio::select! {
                _ = self.delay(initial_delay) => {},
                _ = shutdown_rx.is_shutdown() => {
                    log::info!("scheduled task {name} is shutdown");
                    return;
                },
            }
        }

        loop {
            log::debug!("executing scheduled task {name}");

            tokio::select! {
                _ = task() => {},
                _ = shutdown_rx.is_shutdown() => break,
            }

            tokio::select! {
                _ = self.delay(delay) => {},
                _ = shutdown_rx.is_shutdown() => break,
            }
        }

        log::info!("scheduled task {name} is shutdown");
    }

    /// Creates and executes a periodic task scheduled at fixed rate.
    ///
    /// The task will execute the first time after the given `initial_delay`, and subsequently
    /// with the given period; that is, executions will commence after `initial_delay` then
    /// `initial_delay+period`, then `initial_delay+2*period`, and so on.
    ///
    /// If any execution of this task takes longer than its period, then subsequent
    /// executions may start late, but will not concurrently execute.
    pub async fn schedule_at_fixed_rate<F: AsyncFnMut()>(
        &self,
        name: &'static str,
        mut task: F,
        shutdown_rx: ShutdownRecv,
        initial_delay: Option<Duration>,
        period: Duration,
    ) {
        assert!(period > Duration::new(0, 0), "`period` must be non-zero.");

        fn calculate_next_on_miss(next: Instant, period: Duration) -> Instant {
            let now = Instant::now();

            if now.saturating_duration_since(next) <= Duration::from_millis(5) {
                // finished in time
                make_instant_from(next, period)
            } else {
                // missed the expected execution time; align the next one
                match now.checked_add(period) {
                    None => far_future(),
                    Some(instant) => {
                        let delta = (now - next).as_nanos() % period.as_nanos();
                        let delta: u64 = delta
                            .try_into()
                            // This operation is practically guaranteed not to
                            // fail, as in order for it to fail, `period` would
                            // have to be longer than `now - next`, and both
                            // would have to be longer than 584 years.
                            //
                            // If it did fail, there's not a good way to pass
                            // the error along to the user, so we just panic.
                            .unwrap_or_else(|_| panic!("too much time has elapsed: {delta}"));
                        let delta = Duration::from_nanos(delta);
                        instant - delta
                    }
                }
            }
        }

        log::info!(
            "start scheduled task {name} at fixed rate {period:?} with initial delay {initial_delay:?}",
        );

        let mut next = Instant::now();
        if let Some(initial_delay) = initial_delay
            && initial_delay > Duration::ZERO
        {
            next = make_instant_from_now(initial_delay);
            tokio::select! {
                _ = self.delay_until(next) => {},
                _ = shutdown_rx.is_shutdown() => {
                    log::info!("scheduled task {name} is shutdown");
                    return;
                }
            }
        }

        loop {
            log::debug!("executing scheduled task {name}");

            tokio::select! {
                _ = task() => {},
                _ = shutdown_rx.is_shutdown() => break,
            }

            next = calculate_next_on_miss(next, period);
            tokio::select! {
                _ = self.delay_until(next) => {},
                _ = shutdown_rx.is_shutdown() => break,
            }
        }

        log::info!("scheduled task {name} is shutdown");
    }

    /// Creates and executes a periodic task scheduled with arbitrary delay.
    ///
    /// The task will execute the first time after the given `initial_delay`, and subsequently
    /// based on the result of the task.
    pub async fn schedule_with_arbitrary_delay<F: AsyncFnMut() -> Instant>(
        &self,
        name: &'static str,
        mut task: F,
        shutdown_rx: ShutdownRecv,
        initial_delay: Option<Duration>,
    ) {
        log::info!("start scheduled task {name} with initial delay {initial_delay:?}",);

        if let Some(initial_delay) = initial_delay
            && initial_delay > Duration::ZERO
        {
            tokio::select! {
                _ = self.delay(initial_delay) => {},
                _ = shutdown_rx.is_shutdown() => {
                    log::info!("scheduled task {name} is shutdown");
                    return;
                }
            }
        }

        loop {
            log::debug!("executing scheduled task {name}");

            let next = tokio::select! {
                next = task() => next,
                _ = shutdown_rx.is_shutdown() => break,
            };

            tokio::select! {
                _ = self.delay_until(next) => {},
                _ = shutdown_rx.is_shutdown() => break,
            }
        }

        log::info!("scheduled task {name} is shutdown");
    }
}
```

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

This crate is built against the latest stable release, and its minimum supported rustc version is 1.85.0.

The policy is that the minimum Rust version required to use this crate can be increased in minor version updates. For example, if Fastimer 1.0 requires Rust 1.20.0, then Fastimer 1.0.z for all values of z will also require Rust 1.20.0 or newer. However, Fastimer 1.y for y > 0 may require a newer minimum version of Rust.

## License

This project is licensed under [Apache License, Version 2.0](LICENSE).
