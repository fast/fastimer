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

//! Fastimer implements runtime-agnostic driver for async timers and scheduled tasks.
//!
//! # Scheduled Tasks
//!
//! Fastimer provides [`ResultAction`] and [`GenericAction`] that can be scheduled as a repeating
//! and cancellable task.
//!
//! ## Examples
//!
//! Schedule a repeating task with [`ResultAction`]:
//!
//! ```rust
//! use std::convert::Infallible;
//! use std::future::Future;
//!
//! use fastimer::tokio::MakeTokioDelay;
//! use fastimer::tokio::TokioSpawn;
//! use fastimer::ResultActionExt;
//! use fastimer::Task;
//!
//! struct TickAction(u32);
//! impl fastimer::ResultAction for TickAction {
//!     type Error = Infallible;
//!
//!     fn name(&self) -> &str {
//!         "tick"
//!     }
//!
//!     async fn run(&mut self) -> Result<(), Self::Error> {
//!         self.0 += 1;
//!         println!("tick: {}", self.0);
//!         Ok(())
//!     }
//! }
//!
//! let tick = TickAction(0);
//! let rt = tokio::runtime::Runtime::new().unwrap();
//! rt.block_on(async move {
//!     let task = tick.schedule_with_fixed_delay(
//!         &TokioSpawn::current(),
//!         MakeTokioDelay,
//!         None,
//!         std::time::Duration::from_secs(1),
//!         false,
//!     );
//!
//!     tokio::time::sleep(std::time::Duration::from_secs(5)).await;
//!     task.cancel();
//!     let _ = task.into_inner().await;
//! });
//! ```
//!
//! # Time Driver
//!
//! [`driver::TimeDriver`] is a runtime-agnostic time driver for creating delay futures. To use the
//! time driver, you need to enable the `driver` feature flag.
//!
//! ## Examples
//!
//! Play with a time driver:
//!
//! ```rust
//! let (mut driver, context, shutdown) = fastimer::driver::driver();
//!
//! std::thread::spawn(move || loop {
//!     if driver.turn() {
//!         break;
//!     }
//! });
//!
//! let delay = context.delay(std::time::Duration::from_secs(1));
//! pollster::block_on(delay); // finish after 1 second
//! shutdown.shutdown();
//! ```

use std::future::Future;
use std::time::Duration;
use std::time::Instant;

mod generic;
pub use generic::*;

mod result;
pub use result::*;

#[cfg(feature = "driver")]
pub mod driver;
#[cfg(any(feature = "tokio-time", feature = "tokio-spawn"))]
pub mod tokio;

// Roughly 30 years from now.
fn far_future() -> Instant {
    // API does not provide a way to obtain max `Instant`
    // or convert specific date in the future to instant.
    // 1000 years overflows on macOS, 100 years overflows on FreeBSD.
    Instant::now() + Duration::from_secs(86400 * 365 * 30)
}

fn make_instant_from(now: Instant, dur: Duration) -> Instant {
    now.checked_add(dur).unwrap_or_else(far_future)
}

fn make_instant_from_now(dur: Duration) -> Instant {
    make_instant_from(Instant::now(), dur)
}

/// A trait for creating delay futures.
pub trait MakeDelay: Send + 'static {
    /// Create a future that completes at the specified instant.
    fn delay_util(&self, at: Instant) -> impl Future<Output = ()> + Send;

    /// Create a future that completes after the specified duration.
    fn delay(&self, duration: Duration) -> impl Future<Output = ()> + Send {
        self.delay_util(make_instant_from_now(duration))
    }
}

/// A cancellable task.
pub trait Task {
    /// Cancel this task.
    fn cancel(&self);
}

/// A trait for spawning futures.
pub trait Spawn {
    /// The type of cancellable task that this spawn produces.
    type Task: Task;

    /// Spawn a future and return a cancellable future.
    fn spawn<F: Future<Output = ()> + Send + 'static>(&self, future: F) -> Self::Task;
}

#[cfg(test)]
fn setup_logging() {
    use std::sync::Once;
    static SETUP_LOGGING: Once = Once::new();
    SETUP_LOGGING.call_once(|| {
        let _ = logforth::builder()
            .dispatch(|d| {
                d.filter(log::LevelFilter::Info)
                    .append(logforth::append::Stderr::default())
            })
            .try_apply();
    });
}
