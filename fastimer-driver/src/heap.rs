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

use std::collections::BinaryHeap;
use std::ops::ControlFlow;
use std::sync::atomic;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use crossbeam_queue::SegQueue;
use parking::Parker;
use parking::Unparker;

use crate::TimeContext;
use crate::TimeDriverShutdown;
use crate::TimeEntry;

/// Returns a new time driver, its time context and the shutdown handle.
pub fn binary_heap_driver() -> (BinaryHeapTimeDriver, TimeContext, TimeDriverShutdown) {
    let (parker, unparker) = parking::pair();
    let timers = BinaryHeap::new();
    let inbounds = Arc::new(SegQueue::new());
    let shutdown = Arc::new(AtomicBool::new(false));

    let driver = BinaryHeapTimeDriver {
        parker,
        unparker,
        timers,
        inbounds,
        shutdown,
    };

    let context = TimeContext {
        unparker: driver.unparker.clone(),
        inbounds: driver.inbounds.clone(),
    };

    let shutdown = TimeDriverShutdown {
        unparker: driver.unparker.clone(),
        shutdown: driver.shutdown.clone(),
    };

    (driver, context, shutdown)
}

/// A heap-based time driver that drives registered timers.
#[derive(Debug)]
pub struct BinaryHeapTimeDriver {
    parker: Parker,
    unparker: Unparker,
    timers: BinaryHeap<TimeEntry>,
    inbounds: Arc<SegQueue<TimeEntry>>,
    shutdown: Arc<AtomicBool>,
}

impl BinaryHeapTimeDriver {
    /// Drives the timers and returns `true` if the driver has been shut down.
    pub fn turn(&mut self) -> ControlFlow<()> {
        if self.shutdown.load(atomic::Ordering::Acquire) {
            return ControlFlow::Break(());
        }

        match self.timers.peek() {
            None => self.parker.park(),
            Some(entry) => {
                let delta = entry.when.saturating_duration_since(Instant::now());
                if delta > Duration::ZERO {
                    self.parker.park_timeout(delta);
                }
            }
        }

        while let Some(entry) = self.inbounds.pop() {
            self.timers.push(entry);
        }

        while let Some(entry) = self.timers.peek() {
            if entry.when <= Instant::now() {
                entry.waker.wake();
                let _ = self.timers.pop();
            } else {
                break;
            }
        }

        if self.shutdown.load(atomic::Ordering::Acquire) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }
}
