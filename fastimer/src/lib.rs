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
//! Fastimer provides [`SimpleAction`] and [`GenericAction`] that can be scheduled as a repeating
//! and cancellable task.
//!
//! ## Examples
//!
//! Schedule a repeating task with [`SimpleAction`]:
//!
//! ```rust
//! use std::sync::atomic::AtomicBool;
//! use std::sync::atomic::Ordering;
//! use std::sync::mpsc;
//! use std::sync::mpsc::Sender;
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! use fastimer::tokio::MakeTokioDelay;
//! use fastimer::tokio::TokioSpawn;
//! use fastimer::SimpleActionExt;
//!
//! struct TickAction {
//!     count: u32,
//!     shutdown: Arc<AtomicBool>,
//!     tx_stopped: Sender<()>,
//! }
//!
//! impl fastimer::SimpleAction for TickAction {
//!     fn name(&self) -> &str {
//!         "tick"
//!     }
//!
//!     async fn run(&mut self) -> bool {
//!         if self.shutdown.load(Ordering::Acquire) {
//!             println!("shutdown");
//!             let _ = self.tx_stopped.send(());
//!             true
//!         } else {
//!             println!("tick: {}", self.count);
//!             self.count += 1;
//!             false
//!         }
//!     }
//! }
//!
//! let (tx, rx) = mpsc::channel();
//! let shutdown = Arc::new(AtomicBool::new(false));
//!
//! let tick = TickAction {
//!     count: 0,
//!     shutdown: shutdown.clone(),
//!     tx_stopped: tx,
//! };
//!
//! let rt = tokio::runtime::Runtime::new().unwrap();
//! rt.block_on(async move {
//!     tick.schedule_with_fixed_delay(
//!         &TokioSpawn::current(),
//!         MakeTokioDelay,
//!         None,
//!         Duration::from_secs(1),
//!     );
//!
//!     tokio::time::sleep(Duration::from_secs(5)).await;
//!     shutdown.store(true, Ordering::Release);
//! });
//! let _ = rx.recv_timeout(Duration::from_secs(5));
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

mod simple;
pub use simple::*;

#[cfg(feature = "driver")]
pub mod driver;
#[cfg(any(feature = "tokio-time", feature = "tokio-spawn"))]
pub mod tokio;

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
    /// Create a future that completes at the specified instant.
    fn delay_util(&self, at: Instant) -> impl Future<Output = ()> + Send;

    /// Create a future that completes after the specified duration.
    fn delay(&self, duration: Duration) -> impl Future<Output = ()> + Send {
        self.delay_util(make_instant_from_now(duration))
    }
}

/// A trait for spawning futures.
pub trait Spawn {
    /// Spawn a future and return a cancellable future.
    fn spawn<F: Future<Output = ()> + Send + 'static>(&self, future: F);
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
