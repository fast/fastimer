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

//! A time driver based on a hierarchical time wheel.

use std::cmp;
use std::collections::VecDeque;
use std::ops::ControlFlow;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::task::Context;
use std::task::Poll;
use std::time::Duration;
use std::time::Instant;

use atomic_waker::AtomicWaker;
use crossbeam_queue::SegQueue;
use fastimer_core::MakeDelay;
use parking::Parker;
use parking::Unparker;

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

impl MakeDelay for TimeContext {
    type Delay = Delay;

    fn delay_until(&self, when: Instant) -> Self::Delay {
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

/// Returns a new time driver, its time context and the shutdown handle.
pub fn time_wheel_driver() -> (TimeWheelTimeDriver, TimeContext, TimeDriverShutdown) {
    let (parker, unparker) = parking::pair();
    let wheel = TimeWheel::new();
    let inbounds = Arc::new(SegQueue::new());
    let shutdown = Arc::new(AtomicBool::new(false));

    let driver = TimeWheelTimeDriver {
        parker,
        unparker,
        wheel,
        inbounds,
        shutdown,
        last_tick: Instant::now(),
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

const WHEEL_BITS: u64 = 6;
const WHEEL_SIZE: u64 = 1 << WHEEL_BITS;
const WHEEL_MASK: u64 = WHEEL_SIZE - 1;
const WHEEL_LEVELS: usize = 4;

type Slot = VecDeque<TimeEntry>;

#[derive(Debug)]
/// The time wheel structure for hierarchical timer management
struct TimeWheel {
    /// Multi-level time wheel, each level has 64 slots
    wheels: [Vec<Slot>; WHEEL_LEVELS],
    /// The current tick count
    current_tick: AtomicU64,
    /// Duration of each tick in milliseconds
    tick_duration: Duration,
}

impl TimeWheel {
    fn new() -> Self {
        let wheels = [(); WHEEL_LEVELS].map(|_| (0..WHEEL_SIZE).map(|_| VecDeque::new()).collect());

        Self {
            wheels,
            current_tick: AtomicU64::new(0),
            tick_duration: Duration::from_millis(1),
        }
    }

    /// Calculate which wheel level and slot a timer should go into.
    fn calculate_slot(&self, when: Instant) -> Option<(usize, usize)> {
        let now = Instant::now();
        if when <= now {
            return None;
        }

        let delta = when.duration_since(now);
        let ticks = delta.as_millis() as u64;
        let current_tick = self.current_tick.load(atomic::Ordering::Acquire);
        let target_tick = current_tick + ticks;

        // Determine which wheel level to place the timer in.
        for level in 0..WHEEL_LEVELS {
            let shift = WHEEL_BITS * level as u64;
            let mask = WHEEL_MASK << shift;

            if (target_tick & mask) != (current_tick & mask) {
                let slot_index = ((target_tick >> shift) & WHEEL_MASK) as usize;
                return Some((level, slot_index));
            }
        }

        // If the delay exceeds all wheel levels, place in the last slot of the highest level.
        Some((WHEEL_LEVELS - 1, (WHEEL_SIZE - 1) as usize))
    }

    /// Add a timer entry into the time wheel.
    fn add_timer(&mut self, entry: TimeEntry) {
        if let Some((level, slot)) = self.calculate_slot(entry.when) {
            self.wheels[level][slot].push_back(entry);
        } else {
            // If already expired, wake immediately.
            entry.waker.wake();
        }
    }

    /// Advance the time wheel by one tick and return all expired timers.
    fn advance_tick(&mut self) -> Vec<TimeEntry> {
        let mut expired = Vec::new();
        let current_tick = self.current_tick.fetch_add(1, atomic::Ordering::AcqRel);

        // Check the current slot in the first-level wheel.
        let slot_index = (current_tick & WHEEL_MASK) as usize;
        expired.extend(self.wheels[0][slot_index].drain(..));

        // Check if timers from higher-level wheels need to be cascaded down.
        for level in 1..WHEEL_LEVELS {
            let shift = WHEEL_BITS * level as u64;
            if (current_tick & ((1 << shift) - 1)) == 0 {
                let slot_index = ((current_tick >> shift) & WHEEL_MASK) as usize;
                let timers: Vec<_> = self.wheels[level][slot_index].drain(..).collect();

                for timer in timers {
                    self.add_timer(timer);
                }
            }
        }

        expired
    }
}

/// A time-wheel based time driver that drives registered timers.
#[derive(Debug)]
pub struct TimeWheelTimeDriver {
    parker: Parker,
    unparker: Unparker,
    wheel: TimeWheel,
    inbounds: Arc<SegQueue<TimeEntry>>,
    shutdown: Arc<AtomicBool>,
    last_tick: Instant,
}

impl TimeWheelTimeDriver {
    /// Drives the timers and returns `true` if the driver has been shut down.
    pub fn turn(&mut self) -> ControlFlow<()> {
        if self.shutdown.load(atomic::Ordering::Acquire) {
            return ControlFlow::Break(());
        }

        let now = Instant::now();

        while let Some(entry) = self.inbounds.pop() {
            self.wheel.add_timer(entry);
        }

        let elapsed = now.duration_since(self.last_tick);
        let ticks_to_advance = (elapsed.as_millis() as u64).max(1);

        let mut all_expired = Vec::new();
        for _ in 0..ticks_to_advance {
            let expired = self.wheel.advance_tick();
            all_expired.extend(expired);
        }

        for entry in all_expired {
            if entry.when <= now {
                entry.waker.wake();
            } else {
                self.wheel.add_timer(entry);
            }
        }

        self.last_tick = now;

        let next_tick_time = self.last_tick + self.wheel.tick_duration;
        let sleep_duration = next_tick_time.saturating_duration_since(Instant::now());

        if sleep_duration > Duration::ZERO {
            self.parker.park_timeout(sleep_duration);
        }

        if self.shutdown.load(atomic::Ordering::Acquire) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }
}
