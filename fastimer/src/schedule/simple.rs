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
use std::time::Duration;
use std::time::Instant;

use crate::far_future;
use crate::make_instant_from;
use crate::make_instant_from_now;
use crate::schedule::select::select;
use crate::schedule::select::Either;
use crate::MakeDelay;
use crate::Spawn;

/// Repeatable scheduled action that returns a result.
pub trait SimpleAction: Send + 'static {
    /// The name of the action.
    fn name(&self) -> &str;

    /// Run the action.
    fn run(&mut self) -> impl Future<Output = ()> + Send;

    /// Return a future that resolves when the action is shutdown.
    ///
    /// By default, this function returns a future that never resolves, i.e., the action will never
    /// be shutdown.
    fn is_shutdown(&self) -> impl Future<Output = ()> + Send {
        std::future::pending()
    }

    /// A teardown hook that is called when the action is shutdown.
    fn teardown(&mut self) {
        #[cfg(feature = "logging")]
        log::debug!("scheduled task {} is stopped", self.name());
    }
}

/// An extension trait for [`SimpleAction`] that provides scheduling methods.
pub trait SimpleActionExt: SimpleAction {
    /// Creates and executes a periodic action that becomes enabled first after the given
    /// `initial_delay`, and subsequently with the given `delay` between the termination of one
    /// execution and the commencement of the next.
    ///
    /// This task will terminate if [`SimpleAction::run`] returns `true`.
    fn schedule_with_fixed_delay<S, D>(
        mut self,
        spawn: &S,
        make_delay: D,
        initial_delay: Option<Duration>,
        delay: Duration,
    ) where
        Self: Sized,
        S: Spawn,
        D: MakeDelay,
    {
        spawn.spawn(async move {
            #[cfg(feature = "logging")]
            log::debug!(
                "start scheduled task {} with fixed delay {:?} and initial delay {:?}",
                self.name(),
                delay,
                initial_delay
            );

            if let Some(initial_delay) = initial_delay {
                if initial_delay > Duration::ZERO {
                    make_delay.delay(initial_delay).await;
                }
            }

            loop {
                #[cfg(feature = "logging")]
                log::debug!("executing scheduled task {}", self.name());
                self.run().await;

                let is_shutdown = self.is_shutdown();
                let delay = make_delay.delay(delay);
                if let Either::Left(()) = select(is_shutdown, delay).await {
                    self.teardown();
                    break;
                }
            }
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
    fn schedule_at_fixed_rate<S, D>(
        mut self,
        spawn: &S,
        make_delay: D,
        initial_delay: Option<Duration>,
        period: Duration,
    ) where
        Self: Sized,
        S: Spawn,
        D: MakeDelay,
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
            #[cfg(feature = "logging")]
            log::debug!(
                "start scheduled task {} at fixed rate {:?} with initial delay {:?}",
                self.name(),
                period,
                initial_delay
            );

            let mut next = Instant::now();
            if let Some(initial_delay) = initial_delay {
                if initial_delay > Duration::ZERO {
                    next = make_instant_from_now(initial_delay);
                    make_delay.delay_util(next).await;
                }
            }

            loop {
                #[cfg(feature = "logging")]
                log::debug!("executing scheduled task {}", self.name());
                self.run().await;

                next = calculate_next_on_miss(next, period);
                let is_shutdown = self.is_shutdown();
                let delay = make_delay.delay_util(next);
                if let Either::Left(()) = select(is_shutdown, delay).await {
                    self.teardown();
                    break;
                }
            }
        });
    }
}

impl<T: SimpleAction> SimpleActionExt for T {}

#[cfg(all(test, feature = "test"))]
mod tests {
    use std::future::Future;
    use std::sync::Arc;
    use std::time::Duration;

    use mea::latch::Latch;
    use mea::waitgroup::WaitGroup;

    use super::SimpleActionExt;
    use crate::setup_logging;
    use crate::tokio::MakeTokioDelay;
    use crate::tokio::TokioSpawn;

    struct TickAction {
        name: String,
        count: u32,
        sleep: Duration,

        latch: Arc<Latch>,
        _wg: WaitGroup,
    }

    impl super::SimpleAction for TickAction {
        fn name(&self) -> &str {
            &self.name
        }

        async fn run(&mut self) {
            self.count += 1;
            log::info!("[{}] tick count: {}", self.name, self.count);
            tokio::time::sleep(self.sleep).await;
        }

        fn is_shutdown(&self) -> impl Future<Output = ()> + Send {
            self.latch.wait()
        }
    }

    #[test]
    fn test_schedule_with_fixed_delay() {
        setup_logging();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let wg = WaitGroup::new();
            let latch = Arc::new(Latch::new(1));

            let action = TickAction {
                name: "fixed-delay".to_string(),
                count: 0,
                sleep: Duration::from_secs(1),
                latch: latch.clone(),
                _wg: wg.clone(),
            };

            action.schedule_with_fixed_delay(
                &TokioSpawn::current(),
                MakeTokioDelay,
                None,
                Duration::from_secs(2),
            );

            tokio::time::sleep(Duration::from_secs(10)).await;
            latch.count_down();
            tokio::time::timeout(Duration::from_secs(5), wg)
                .await
                .unwrap();
        });
    }

    #[test]
    fn test_schedule_at_fixed_rate() {
        setup_logging();

        let rt = tokio::runtime::Runtime::new().unwrap();

        async fn do_schedule_at_fixed_rate(sleep: u64, period: u64) {
            let wg = WaitGroup::new();
            let latch = Arc::new(Latch::new(1));

            let action = TickAction {
                name: format!("fixed-rate-{sleep}/{period}"),
                count: 0,
                sleep: Duration::from_secs(sleep),
                latch: latch.clone(),
                _wg: wg.clone(),
            };

            action.schedule_at_fixed_rate(
                &TokioSpawn::current(),
                MakeTokioDelay,
                None,
                Duration::from_secs(period),
            );

            tokio::time::sleep(Duration::from_secs(10)).await;
            latch.count_down();
            tokio::time::timeout(Duration::from_secs(5), wg)
                .await
                .unwrap();
        }

        rt.block_on(do_schedule_at_fixed_rate(1, 2));
        rt.block_on(do_schedule_at_fixed_rate(3, 2));
        rt.block_on(do_schedule_at_fixed_rate(5, 2));
    }
}
