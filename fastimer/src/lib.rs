// Copyright 2024 FastLabs Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! Fastimer implements runtime-agnostic driver for async timers and scheduled actions.
//!
//! # Scheduled Actions
//!
//! Fastimer provides [`SimpleAction`] and [`ArbitraryDelayAction`] that can be
//! scheduled as a repeating and cancellable action.
//!
//! # Timeout
//!
//! [`Timeout`] is a future combinator that completes when the inner future completes or when the
//! timeout expires.
//!
//! # Time Driver
//!
//! [`TimeDriver`] is a runtime-agnostic time driver for creating delay futures. To use the
//! time driver, you need to enable the `driver` feature flag.
//!
//! [`SimpleAction`]: schedule::SimpleAction
//! [`ArbitraryDelayAction`]: schedule::ArbitraryDelayAction
//! [`TimeDriver`]: driver::TimeDriver

use std::future::Future;
use std::future::IntoFuture;
use std::task::Context;
use std::task::Poll;
use std::time::Duration;
use std::time::Instant;

mod macros;
pub mod schedule;

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

/// A trait for creating delay futures.
pub trait MakeDelay: Send + 'static {
    /// The future returned by the `delay`/`delay_until` method.
    type Delay: Future<Output = ()> + Send;

    /// Create a future that completes at the specified instant.
    fn delay_util(&self, at: Instant) -> Self::Delay;

    /// Create a future that completes after the specified duration.
    fn delay(&self, duration: Duration) -> Self::Delay {
        self.delay_util(make_instant_from_now(duration))
    }
}

/// A trait for spawning futures.
pub trait Spawn {
    /// Spawn a future and return a cancellable future.
    fn spawn<F: Future<Output = ()> + Send + 'static>(&self, future: F);
}

/// Errors returned by [`Timeout`].
///
/// This error is returned when a timeout expires before the function was able
/// to finish.
#[derive(Debug, PartialEq, Eq)]
pub struct Elapsed(());

/// A future that completes when the inner future completes or when the timeout expires.
///
/// Created by [`timeout`] or [`timeout_at`].
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[derive(Debug)]
#[pin_project::pin_project]
pub struct Timeout<T, D> {
    #[pin]
    value: T,
    #[pin]
    delay: D,
}

impl<T, D> Timeout<T, D> {
    /// Gets a reference to the underlying value in this timeout.
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Gets a mutable reference to the underlying value in this timeout.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }

    /// Consumes this timeout, returning the underlying value.
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T, D> Future for Timeout<T, D>
where
    T: Future,
    D: Future<Output = ()>,
{
    type Output = Result<T::Output, Elapsed>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        if let Poll::Ready(v) = this.value.poll(cx) {
            return Poll::Ready(Ok(v));
        }

        match this.delay.poll(cx) {
            Poll::Ready(()) => Poll::Ready(Err(Elapsed(()))),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Requires a `Future` to complete before the specified duration has elapsed.
pub fn timeout<F, D>(
    duration: Duration,
    future: F,
    make_delay: D,
) -> Timeout<F::IntoFuture, D::Delay>
where
    F: IntoFuture,
    D: MakeDelay,
{
    let delay = make_delay.delay(duration);
    Timeout {
        value: future.into_future(),
        delay,
    }
}

/// Requires a `Future` to complete before the specified instant in time.
pub fn timeout_at<F, D>(
    deadline: Instant,
    future: F,
    make_delay: D,
) -> Timeout<F::IntoFuture, D::Delay>
where
    F: IntoFuture,
    D: MakeDelay,
{
    let delay = make_delay.delay_util(deadline);
    Timeout {
        value: future.into_future(),
        delay,
    }
}
