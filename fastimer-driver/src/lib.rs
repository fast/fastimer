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
#![deny(missing_docs)]

//! Runtime-agnostic time driver for creating delay futures.

use std::cmp;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic;
use std::sync::atomic::AtomicBool;
use std::task::Context;
use std::task::Poll;
use std::time::Duration;
use std::time::Instant;

use atomic_waker::AtomicWaker;
use crossbeam_queue::SegQueue;
use fastimer::MakeDelay;
use fastimer::make_instant_from_now;
use parking::Unparker;

mod heap;
pub use heap::*;

#[derive(Debug)]
struct TimeEntry {
    when: Instant,
    waker: Arc<AtomicWaker>,
}

impl PartialEq for TimeEntry {
    fn eq(&self, other: &Self) -> bool {
        self.when == other.when
    }
}

impl Eq for TimeEntry {}

impl PartialOrd for TimeEntry {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimeEntry {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.when.cmp(&other.when)
    }
}

/// Future returned by [`delay`] and [`delay_until`].
///
/// [`delay`]: TimeContext::delay
/// [`delay_until`]: TimeContext::delay_until
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[derive(Debug)]
pub struct Delay {
    when: Instant,
    waker: Arc<AtomicWaker>,
}

impl Future for Delay {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Instant::now() >= self.when {
            self.waker.take();
            Poll::Ready(())
        } else {
            self.waker.register(cx.waker());
            Poll::Pending
        }
    }
}

impl Drop for Delay {
    fn drop(&mut self) {
        self.waker.take();
    }
}

/// A time context for creating [`Delay`]s.
#[derive(Debug, Clone)]
pub struct TimeContext {
    unparker: Unparker,
    inbounds: Arc<SegQueue<TimeEntry>>,
}

impl TimeContext {
    /// Returns a future that completes after the specified duration.
    pub fn delay(&self, dur: Duration) -> Delay {
        self.delay_until(make_instant_from_now(dur))
    }

    /// Returns a future that completes at the specified instant.
    pub fn delay_until(&self, when: Instant) -> Delay {
        let waker = Arc::new(AtomicWaker::new());
        let delay = Delay {
            when,
            waker: waker.clone(),
        };
        self.inbounds.push(TimeEntry { when, waker });
        self.unparker.unpark();
        delay
    }
}

/// A handle to shut down the time driver.
#[derive(Debug, Clone)]
pub struct TimeDriverShutdown {
    unparker: Unparker,
    shutdown: Arc<AtomicBool>,
}

impl TimeDriverShutdown {
    /// Shuts down the time driver.
    pub fn shutdown(&self) {
        self.shutdown.store(true, atomic::Ordering::Release);
        self.unparker.unpark();
    }
}

/// A delay implementation that uses the given time context.
#[derive(Debug, Clone)]
pub struct MakeFastimerDelay(TimeContext);

impl MakeFastimerDelay {
    /// Create a new [`MakeFastimerDelay`] with the given [`TimeContext`].
    pub fn new(context: TimeContext) -> Self {
        MakeFastimerDelay(context)
    }
}

impl MakeDelay for MakeFastimerDelay {
    type Delay = Delay;

    fn delay_until(&self, at: Instant) -> Self::Delay {
        self.0.delay_until(at)
    }

    fn delay(&self, duration: Duration) -> Self::Delay {
        self.0.delay(duration)
    }
}
