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
use crate::MakeDelay;
use crate::Spawn;

/// Repeatable scheduled action that returns a result.
pub trait SimpleAction: Send + 'static {
    /// The name of the action.
    fn name(&self) -> &str;

    /// Run the action.
    ///
    /// Return `true` if this action should stop; `false` otherwise.
    fn run(&mut self) -> impl Future<Output = bool> + Send;
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
                if self.run().await {
                    break;
                }
                make_delay.delay(delay).await;
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

        fn calculate_next_on_miss(next: Instant, now: Instant, period: Duration) -> Instant {
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
                if self.run().await {
                    break;
                }

                let now = Instant::now();
                if now.saturating_duration_since(next) > Duration::from_millis(5) {
                    // align the next execution time if we miss the expected one
                    next = calculate_next_on_miss(next, now, period);
                } else {
                    // self.run() finishes in time
                    next = make_instant_from(next, period);
                }
                make_delay.delay_util(next).await;
            }
        });
    }
}

impl<T: SimpleAction> SimpleActionExt for T {}

#[cfg(all(test, feature = "test"))]
mod tests {
    use std::time::Duration;

    use tokio::sync::mpsc::Receiver;
    use tokio::sync::mpsc::Sender;

    use crate::setup_logging;
    use crate::tokio::MakeTokioDelay;
    use crate::tokio::TokioSpawn;
    use crate::SimpleActionExt;

    struct TickAction {
        name: String,
        count: u32,
        sleep: Duration,

        tx_stopped: Sender<()>,
        rx_shutdown: Receiver<()>,
    }

    impl super::SimpleAction for TickAction {
        fn name(&self) -> &str {
            &self.name
        }

        async fn run(&mut self) -> bool {
            self.count += 1;
            log::info!("[{}] tick count: {}", self.name, self.count);

            tokio::select! {
                _ = tokio::time::sleep(self.sleep) => false,
                _ = self.rx_shutdown.recv() => {
                    log::info!("[{}] shutdown", self.name);
                    let _ = self.tx_stopped.send(()).await;
                    true
                },
            }
        }
    }

    #[test]
    fn test_schedule_with_fixed_delay() {
        setup_logging();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let (tx_shutdown, rx_shutdown) = tokio::sync::mpsc::channel(1);
            let (tx_stopped, mut rx_stopped) = tokio::sync::mpsc::channel(1);

            let action = TickAction {
                name: "fixed-delay".to_string(),
                count: 0,
                sleep: Duration::from_secs(1),
                tx_stopped,
                rx_shutdown,
            };

            action.schedule_with_fixed_delay(
                &TokioSpawn::current(),
                MakeTokioDelay,
                None,
                Duration::from_secs(2),
            );

            tokio::time::sleep(Duration::from_secs(10)).await;
            let _ = tx_shutdown.send(()).await;
            rx_stopped.recv().await;
        });
    }

    #[test]
    fn test_schedule_at_fixed_rate() {
        setup_logging();

        let rt = tokio::runtime::Runtime::new().unwrap();

        async fn do_schedule_at_fixed_rate(sleep: u64, period: u64) {
            let (tx_shutdown, rx_shutdown) = tokio::sync::mpsc::channel(1);
            let (tx_stopped, mut rx_stopped) = tokio::sync::mpsc::channel(1);

            let action = TickAction {
                name: format!("fixed-rate-{sleep}/{period}"),
                count: 0,
                sleep: Duration::from_secs(sleep),
                tx_stopped,
                rx_shutdown,
            };

            action.schedule_at_fixed_rate(
                &TokioSpawn::current(),
                MakeTokioDelay,
                None,
                Duration::from_secs(period),
            );

            tokio::time::sleep(Duration::from_secs(10)).await;
            let _ = tx_shutdown.send(()).await;
            rx_stopped.recv().await;
        }

        rt.block_on(do_schedule_at_fixed_rate(1, 2));
        rt.block_on(do_schedule_at_fixed_rate(3, 2));
        rt.block_on(do_schedule_at_fixed_rate(5, 2));
    }
}
