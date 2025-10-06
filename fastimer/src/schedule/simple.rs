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

use std::future::Future;
use std::pin::pin;
use std::time::Duration;
use std::time::Instant;

use super::execute_or_shutdown;
use crate::MakeDelay;
use crate::Spawn;
use crate::debug;
use crate::far_future;
use crate::info;
use crate::make_instant_from;
use crate::make_instant_from_now;

/// Repeatable action.
///
/// See [`SimpleActionExt`] for scheduling methods.
pub trait SimpleAction: Send + 'static {
    /// The name of the trait.
    fn name(&self) -> &str;

    /// Run the action.
    fn run(&mut self) -> impl Future<Output = ()> + Send;
}

/// An extension trait for [`SimpleAction`] that provides scheduling methods.
pub trait SimpleActionExt: SimpleAction {
    /// Creates and executes a periodic action that becomes enabled first after the given
    /// `initial_delay`, and subsequently with the given `delay` between the termination of one
    /// execution and the commencement of the next.
    ///
    /// This task will terminate if [`SimpleAction::run`] returns `true`.
    fn schedule_with_fixed_delay<Fut, S, D>(
        mut self,
        is_shutdown: Fut,
        spawn: &S,
        make_delay: D,
        initial_delay: Option<Duration>,
        delay: Duration,
    ) where
        Self: Sized,
        Fut: Future<Output = ()> + Send + 'static,
        S: Spawn,
        D: MakeDelay + Send + 'static,
    {
        spawn.spawn(async move {
            info!(
                "start scheduled task {} with fixed delay {:?} and initial delay {:?}",
                self.name(),
                delay,
                initial_delay
            );

            let mut is_shutdown = pin!(is_shutdown);
            'schedule: {
                if let Some(initial_delay) = initial_delay {
                    if initial_delay > Duration::ZERO
                        && execute_or_shutdown(make_delay.delay(initial_delay), &mut is_shutdown)
                            .await
                            .is_break()
                    {
                        break 'schedule;
                    }
                }

                loop {
                    debug!("executing scheduled task {}", self.name());
                    if execute_or_shutdown(self.run(), &mut is_shutdown)
                        .await
                        .is_break()
                    {
                        break;
                    };

                    if execute_or_shutdown(make_delay.delay(delay), &mut is_shutdown)
                        .await
                        .is_break()
                    {
                        break;
                    }
                }
            }

            info!("scheduled task {} is shutdown", self.name());
        });
    }

    /// Creates and executes a periodic action that becomes enabled first after the given
    /// `initial_delay`, and subsequently with the given period; that is executions will commence
    /// after `initial_delay` then `initial_delay+period`, then `initial_delay+2*period`, and so
    /// on.
    ///
    /// This task will terminate if [`SimpleAction::run`] returns `true`.
    ///
    /// If any execution of this task takes longer than its period, then subsequent
    /// executions may start late, but will not concurrently execute.
    fn schedule_at_fixed_rate<Fut, S, D>(
        mut self,
        is_shutdown: Fut,
        spawn: &S,
        make_delay: D,
        initial_delay: Option<Duration>,
        period: Duration,
    ) where
        Self: Sized,
        Fut: Future<Output = ()> + Send + 'static,
        S: Spawn,
        D: MakeDelay + Send + 'static,
    {
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

        spawn.spawn(async move {
            info!(
                "start scheduled task {} at fixed rate {:?} with initial delay {:?}",
                self.name(),
                period,
                initial_delay
            );

            let mut is_shutdown = pin!(is_shutdown);
            'schedule: {
                let mut next = Instant::now();
                if let Some(initial_delay) = initial_delay {
                    if initial_delay > Duration::ZERO {
                        next = make_instant_from_now(initial_delay);
                        if execute_or_shutdown(make_delay.delay_until(next), &mut is_shutdown)
                            .await
                            .is_break()
                        {
                            break 'schedule;
                        }
                    }
                }

                loop {
                    debug!("executing scheduled task {}", self.name());
                    if execute_or_shutdown(self.run(), &mut is_shutdown)
                        .await
                        .is_break()
                    {
                        break;
                    };

                    next = calculate_next_on_miss(next, period);
                    if execute_or_shutdown(make_delay.delay_until(next), &mut is_shutdown)
                        .await
                        .is_break()
                    {
                        break;
                    }
                }
            }

            info!("scheduled task {} is shutdown", self.name());
        });
    }
}

impl<T: SimpleAction> SimpleActionExt for T {}
