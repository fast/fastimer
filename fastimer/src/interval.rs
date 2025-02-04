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

// This first version of this file is copied from tokio::time::interval [1], while
// fastimer modifies the code to runtime-agnostic.
//
// [1] https://github.com/tokio-rs/tokio/blob/b8ac94ed/tokio/src/time/interval.rs

use std::future::poll_fn;
use std::future::Future;
use std::pin::Pin;
use std::task::ready;
use std::task::Context;
use std::task::Poll;
use std::time::Duration;
use std::time::Instant;

use crate::far_future;
use crate::MakeDelay;

/// Creates new [`Interval`] that yields with interval of `period`. The first
/// tick completes immediately. The default [`MissedTickBehavior`] is
/// [`Burst`](MissedTickBehavior::Burst), but this can be configured
/// by calling [`set_missed_tick_behavior`](Interval::set_missed_tick_behavior).
///
/// An interval will tick indefinitely. At any time, the [`Interval`] value can
/// be dropped. This cancels the interval.
///
/// # Panics
///
/// This function panics if `period` is zero.
///
/// # Examples
///
/// ```
/// # #[tokio::main]
/// # async fn main() {
/// use std::time::Duration;
/// use std::time::Instant;
///
/// use fastimer::interval;
/// use fastimer::MakeDelay;
///
/// struct TokioDelay;
/// impl MakeDelay for TokioDelay {
///     type Delay = tokio::time::Sleep;
///     fn delay_util(&self, until: Instant) -> Self::Delay {
///         tokio::time::sleep_until(tokio::time::Instant::from_std(until))
///     }
/// }
///
/// let mut interval = interval(Duration::from_millis(10), TokioDelay);
///
/// interval.tick().await; // ticks immediately
/// interval.tick().await; // ticks after 10ms
/// interval.tick().await; // ticks after 10ms
///
/// // approximately 20ms have elapsed.
/// # }
/// ```
///
/// A simple example using `interval` to execute a task every two seconds.
///
/// The difference between `interval` and [`delay`] is that an [`Interval`]
/// measures the time since the last tick, which means that [`.tick().await`]
/// may wait for a shorter time than the duration specified for the interval
/// if some time has passed between calls to [`.tick().await`].
///
/// If the tick in the example below was replaced with [`delay`], the task
/// would only be executed once every three seconds, and not every two
/// seconds.
///
/// ```
/// use std::time::Duration;
/// use std::time::Instant;
///
/// use fastimer::interval;
/// use fastimer::MakeDelay;
///
/// struct TokioDelay;
/// impl MakeDelay for TokioDelay {
///     type Delay = tokio::time::Sleep;
///     fn delay_util(&self, until: Instant) -> Self::Delay {
///         tokio::time::sleep_until(tokio::time::Instant::from_std(until))
///     }
/// }
///
/// async fn task_that_takes_a_second() {
///     println!("hello");
///     TokioDelay.delay(Duration::from_secs(1)).await;
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let mut interval = fastimer::interval(Duration::from_secs(2), TokioDelay);
///     for _ in 0..5 {
///         interval.tick().await;
///         task_that_takes_a_second().await;
///     }
/// }
/// ```
///
/// [`delay`]: MakeDelay::delay
/// [`.tick().await`]: Interval::tick
#[track_caller]
pub fn interval<D: MakeDelay>(period: Duration, make_delay: D) -> Interval<D> {
    assert!(period > Duration::ZERO, "`period` must be non-zero.");
    make_interval(Instant::now(), period, make_delay)
}

/// Creates new [`Interval`] that yields with interval of `period` with the
/// first tick completing at `start`. The default [`MissedTickBehavior`] is
/// [`Burst`](MissedTickBehavior::Burst), but this can be configured
/// by calling [`set_missed_tick_behavior`](Interval::set_missed_tick_behavior).
///
/// An interval will tick indefinitely. At any time, the [`Interval`] value can
/// be dropped. This cancels the interval.
///
/// # Panics
///
/// This function panics if `period` is zero.
///
/// # Examples
///
/// ```
/// # #[tokio::main]
/// # async fn main() {
/// use std::time::Duration;
/// use std::time::Instant;
///
/// use fastimer::interval_at;
/// use fastimer::MakeDelay;
///
/// struct TokioDelay;
/// impl MakeDelay for TokioDelay {
///     type Delay = tokio::time::Sleep;
///     fn delay_util(&self, until: Instant) -> Self::Delay {
///         tokio::time::sleep_until(tokio::time::Instant::from_std(until))
///     }
/// }
///
/// let start = Instant::now() + Duration::from_millis(50);
/// let mut interval = interval_at(start, Duration::from_millis(10), TokioDelay);
///
/// interval.tick().await; // ticks after 50ms
/// interval.tick().await; // ticks after 10ms
/// interval.tick().await; // ticks after 10ms
///
/// // approximately 70ms have elapsed.
/// # }
/// ```
#[track_caller]
pub fn interval_at<D: MakeDelay>(start: Instant, period: Duration, make_delay: D) -> Interval<D> {
    assert!(period > Duration::ZERO, "`period` must be non-zero.");
    make_interval(start, period, make_delay)
}

fn make_interval<D: MakeDelay>(start: Instant, period: Duration, make_delay: D) -> Interval<D> {
    let deadline = start;
    let delay = Box::pin(make_delay.delay_util(start));
    Interval {
        deadline,
        period,
        make_delay,
        delay,
        missed_tick_behavior: MissedTickBehavior::Burst,
    }
}

/// Interval returned by [`interval`] and [`interval_at`].
///
/// This type allows you to wait on a sequence of instants with a certain
/// duration between each instant. Unlike calling [`delay`] in a loop, this lets
/// you count the time spent between the calls to [`delay`] as well.
///
/// [`delay`]: MakeDelay::delay
#[derive(Debug)]
pub struct Interval<D: MakeDelay> {
    /// The next instant when this interval was scheduled to tick.
    deadline: Instant,

    /// The duration between values yielded by this interval.
    period: Duration,

    /// The make delay instance used to create the delay future.
    make_delay: D,

    /// The delay future of the next tick.
    delay: Pin<Box<D::Delay>>,

    /// The strategy Interval should use when a tick is missed.
    missed_tick_behavior: MissedTickBehavior,
}

impl<D: MakeDelay> Interval<D> {
    /// Sets the [`MissedTickBehavior`] strategy that should be used.
    pub fn set_missed_tick_behavior(&mut self, behavior: MissedTickBehavior) {
        self.missed_tick_behavior = behavior;
    }

    /// Returns the [`MissedTickBehavior`] strategy currently being used.
    pub fn missed_tick_behavior(&self) -> MissedTickBehavior {
        self.missed_tick_behavior
    }

    /// Returns the period of the interval.
    pub fn period(&self) -> Duration {
        self.period
    }

    /// Completes when the next instant in the interval has been reached.
    ///
    /// # Cancel safety
    ///
    /// This method is cancellation safe. If `tick` is used as the branch in a `tokio::select!` and
    /// another branch completes first, then no tick has been consumed.
    pub async fn tick(&mut self) -> Instant {
        poll_fn(|cx| self.poll_tick(cx)).await
    }

    /// Polls for the next instant in the interval to be reached.
    ///
    /// This method can return the following values:
    ///
    ///  * `Poll::Pending` if the next instant has not yet been reached.
    ///  * `Poll::Ready(instant)` if the next instant has been reached.
    ///
    /// When this method returns `Poll::Pending`, the current task is scheduled
    /// to receive a wakeup when the instant has elapsed. Note that on multiple
    /// calls to `poll_tick`, only the [`Waker`](std::task::Waker) from the
    /// [`Context`] passed to the most recent call is scheduled to receive a
    /// wakeup.
    pub fn poll_tick(&mut self, cx: &mut Context<'_>) -> Poll<Instant> {
        // Wait for the delay to be done
        ready!(Pin::new(&mut self.delay).poll(cx));

        // Get the time when we were scheduled to tick
        let timeout = self.deadline;

        let now = Instant::now();

        // If a tick was not missed, and thus we are being called before the
        // next tick is due, just schedule the next tick normally, one `period`
        // after `timeout`
        //
        // However, if a tick took excessively long, and we are now behind,
        // schedule the next tick according to how the user specified with
        // `MissedTickBehavior`
        let next = if now > timeout + Duration::from_millis(5) {
            self.missed_tick_behavior
                .next_timeout(timeout, now, self.period)
        } else {
            timeout.checked_add(self.period).unwrap_or_else(far_future)
        };

        // When we arrive here, the internal delay returned `Poll::Ready`.
        // Reassign the delay but do not register it. It should be registered with
        // the next call to `poll_tick`.
        self.delay = Box::pin(self.make_delay.delay_util(next));

        // Return the time when we were scheduled to tick
        Poll::Ready(timeout)
    }
}

/// Defines the behavior of an [`Interval`] when it misses a tick.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissedTickBehavior {
    #[default]
    /// Ticks as fast as possible until caught up.
    Burst,
    /// Ticks at multiples of period from when [`tick`] was called, rather than from start.
    ///
    /// [`tick`]: Interval::tick
    Delay,
    /// Skips missed ticks and tick on the next multiple of period from start.
    Skip,
}

impl MissedTickBehavior {
    /// If a tick is missed, this method is called to determine when the next tick should happen.
    fn next_timeout(&self, timeout: Instant, now: Instant, period: Duration) -> Instant {
        match self {
            Self::Burst => timeout + period,
            Self::Delay => now + period,
            Self::Skip => {
                now + period
                    - Duration::from_nanos(
                        ((now - timeout).as_nanos() % period.as_nanos())
                            .try_into()
                            // This operation is practically guaranteed not to
                            // fail, as in order for it to fail, `period` would
                            // have to be longer than `now - timeout`, and both
                            // would have to be longer than 584 years.
                            //
                            // If it did fail, there's not a good way to pass
                            // the error along to the user, so we just panic.
                            .expect(
                                "too much time has elapsed since the interval was supposed to tick",
                            ),
                    )
            }
        }
    }
}
