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

//! # Fastimer Core APIs
//!
//! Core traits:
//!
//! * [`MakeDelay`]: a trait for creating delay futures.
//! * [`Spawn`]: a trait for spawning futures, this is useful for scheduling tasks.
//!
//! Utility functions:
//!
//! * [`far_future`]: create a far future instant.
//! * [`make_instant_from`]: create an instant from the given instant and a duration.
//! * [`make_instant_from_now`]: create an instant from [`Instant::now`] and a duration.

use std::future::Future;
use std::time::Duration;
use std::time::Instant;

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
pub trait MakeDelay {
    /// The future returned by the `delay`/`delay_until` method.
    type Delay: Future<Output = ()> + Send;

    /// Create a future that completes at the specified instant.
    fn delay_until(&self, at: Instant) -> Self::Delay;

    /// Create a future that completes after the specified duration.
    fn delay(&self, duration: Duration) -> Self::Delay {
        self.delay_until(make_instant_from_now(duration))
    }
}

/// A trait for spawning futures.
pub trait Spawn {
    /// Spawn a future and return a cancellable future.
    fn spawn<F: Future<Output = ()> + Send + 'static>(&self, future: F);
}
