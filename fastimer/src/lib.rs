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

//! Fastimer implements runtime-agnostic timer traits and utilities.
//!
//! # Scheduled Actions
//!
//! Fastimer provides scheduled actions that can be scheduled as a repeating and cancellable action.
//!
//! * [`SimpleAction`]: A simple repeatable action that can be scheduled with a fixed delay, or at a
//!   fixed rate.
//! * [`ArbitraryDelayAction`]: A repeatable action that can be scheduled with arbitrary delay.
//! * [`NotifyAction`]: A repeatable action that can be scheduled by notifications.
//!
//! # Timeout
//!
//! [`Timeout`] is a future combinator that completes when the inner future completes or when the
//! timeout expires.
//!
//! # Interval
//!
//! [`Interval`] ticks at a sequence of instants with a certain duration between each instant.
//!
//! [`SimpleAction`]: schedule::SimpleAction
//! [`ArbitraryDelayAction`]: schedule::ArbitraryDelayAction
//! [`NotifyAction`]: schedule::NotifyAction

#![cfg_attr(docsrs, feature(doc_cfg))]

use std::future::Future;
use std::time::Duration;
use std::time::Instant;

mod interval;
pub use interval::*;

mod timeout;
pub use timeout::*;

pub mod schedule;

// Re-export core traits and functions
pub use fastimer_core::MakeDelay;
pub use fastimer_core::Spawn;
pub use fastimer_core::far_future;
pub use fastimer_core::make_instant_from;
pub use fastimer_core::make_instant_from_now;

/// Provides extension methods for [`MakeDelay`] implementors.
pub trait MakeDelayExt: MakeDelay {
    /// Requires a `Future` to complete before the specified duration has elapsed.
    fn timeout<F: Future>(&self, duration: Duration, fut: F) -> Timeout<F, Self::Delay> {
        timeout(duration, fut, self)
    }

    /// Requires a `Future` to complete before the specified instant in time.
    fn timeout_at<F: Future>(&self, deadline: Instant, fut: F) -> Timeout<F, Self::Delay> {
        timeout_at(deadline, fut, self)
    }

    /// Creates new [`Interval`] that yields with interval of `period`.
    ///
    /// See [`interval`] for more details.
    fn interval(self, period: Duration) -> Interval<Self>
    where
        Self: Sized,
    {
        interval(period, self)
    }

    /// Creates new [`Interval`] that yields with interval of `period` and starts at `at`.
    ///
    /// See [`interval_at`] for more details.
    fn interval_at(self, at: Instant, period: Duration) -> Interval<Self>
    where
        Self: Sized,
    {
        interval_at(at, period, self)
    }
}

impl<T: MakeDelay> MakeDelayExt for T {}

pub(crate) use self::macros::debug;
pub(crate) use self::macros::info;

#[cfg(any(test, feature = "logging"))]
mod macros {
    macro_rules! debug {
        (target: $target:expr, $($arg:tt)+) => (log::debug!(target: $target, $($arg)+));
        ($($arg:tt)+) => (log::debug!($($arg)+));
    }

    macro_rules! info {
        (target: $target:expr, $($arg:tt)+) => (log::info!(target: $target, $($arg)+));
        ($($arg:tt)+) => (log::info!($($arg)+));
    }

    pub(crate) use debug;
    pub(crate) use info;
}

#[cfg(not(any(test, feature = "logging")))]
mod macros {
    macro_rules! info {
        (target: $target:expr, $($arg:tt)+) => {};
        ($($arg:tt)+) => {};
    }

    macro_rules! debug {
        (target: $target:expr, $($arg:tt)+) => {};
        ($($arg:tt)+) => {};
    }

    pub(crate) use debug;
    pub(crate) use info;
}
