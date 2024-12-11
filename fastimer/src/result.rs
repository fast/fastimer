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

pub trait ResultAction: Send + 'static {
    type Error: std::error::Error;

    fn run(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

pub trait ResultActionExt: ResultAction {
    fn schedule_with_fixed_delay<N, S, D>(
        mut self,
        name: N,
        spawn: &S,
        make_delay: D,
        initial_delay: Option<Duration>,
        delay: Duration,
        break_on_error: bool,
    ) -> S::Task
    where
        Self: Sized,
        N: Into<String>,
        S: Spawn,
        D: MakeDelay,
    {
        let name = name.into();
        spawn.spawn(async move {
            log::debug!("start scheduled task {name} with fixed delay {delay:?} and initial delay {initial_delay:?}");

            if let Some(initial_delay) = initial_delay {
                if initial_delay > Duration::ZERO {
                    make_delay.delay(make_instant_from_now(initial_delay)).await;
                }
            }

            loop {
                log::debug!("executing scheduled task {name}");

                if let Err(err) = self.run().await {
                    log::error!("failed to run scheduled task {name}, error: {err}");

                    if break_on_error {
                        break;
                    }
                }

                make_delay.delay(make_instant_from_now(delay)).await;
            }
        })
    }

    fn schedule_at_fixed_rate<N, S, D>(
        mut self,
        name: N,
        spawn: &S,
        make_delay: D,
        initial_delay: Option<Duration>,
        period: Duration,
        break_on_error: bool,
    ) -> S::Task
    where
        Self: Sized,
        N: Into<String>,
        S: Spawn,
        D: MakeDelay,
    {
        assert!(period > Duration::new(0, 0), "`period` must be non-zero.");

        let name = name.into();
        spawn.spawn(async move {
            log::debug!("start scheduled task {name} at fixed rate {period:?} with initial delay {initial_delay:?}");

            let mut next = Instant::now();
            if let Some(initial_delay) = initial_delay {
                if initial_delay > Duration::ZERO {
                    next = make_instant_from_now(initial_delay);
                    make_delay.delay(next).await;
                }
            }

            loop {
                let now = Instant::now();
                let epsilon = Duration::from_millis(5);
                if now.saturating_duration_since(next) > epsilon {
                    next = match now.checked_add(period) {
                        Some(instant) => {
                            let delta = Duration::from_nanos(
                                ((now - next).as_nanos() % period.as_nanos())
                                    .try_into()
                                    // This operation is practically guaranteed not to
                                    // fail, as in order for it to fail, `period` would
                                    // have to be longer than `now - next`, and both
                                    // would have to be longer than 584 years.
                                    //
                                    // If it did fail, there's not a good way to pass
                                    // the error along to the user, so we just panic.
                                    .expect(
                                        "too much time has elapsed since the interval was supposed to tick",
                                    ),
                            );
                            instant - delta
                        },
                        None => far_future(),
                    };
                    make_delay.delay(next).await;
                }

                log::debug!("executing scheduled task {name}");

                if let Err(err) = self.run().await {
                    log::error!("failed to run scheduled task {name}, error: {err}");

                    if break_on_error {
                        break;
                    }
                }

                next = make_instant_from(next, period);
                make_delay.delay(next).await;
            }
        })
    }
}

impl<T: ResultAction> ResultActionExt for T {}

#[cfg(all(test, feature = "test"))]
mod tests {
    use std::convert::Infallible;
    use std::sync::Once;
    use std::time::Duration;

    use crate::MakeTokioDelay;
    use crate::ResultActionExt;
    use crate::Task;
    use crate::TokioSpawn;

    fn setup_logging() {
        static SETUP_LOGGING: Once = Once::new();
        SETUP_LOGGING.call_once(|| {
            let _ = logforth::builder()
                .dispatch(|d| d.append(logforth::append::Stderr::default()))
                .try_apply();
        });
    }

    struct TickAction {
        name: String,
        count: u32,
        sleep: Duration,
    }

    impl super::ResultAction for TickAction {
        type Error = Infallible;

        async fn run(&mut self) -> Result<(), Self::Error> {
            self.count += 1;
            log::info!("[{}] tick count: {}", self.name, self.count);
            tokio::time::sleep(self.sleep).await;
            Ok(())
        }
    }

    #[test]
    fn test_schedule_with_fixed_delay() {
        setup_logging();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let name = "tick-with-fixed-delay";
            let action = TickAction {
                name: name.to_string(),
                count: 0,
                sleep: Duration::from_secs(1),
            };

            let task = action.schedule_with_fixed_delay(
                name,
                &TokioSpawn::current(),
                MakeTokioDelay,
                None,
                Duration::from_secs(2),
                false,
            );

            tokio::time::sleep(Duration::from_secs(10)).await;
            task.cancel();
            let _ = task.into_inner().await;
        });
    }

    #[test]
    fn test_schedule_at_fixed_rate() {
        setup_logging();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let name = "tick-at-fixed-rate-1/2";
            let action = TickAction {
                name: name.to_string(),
                count: 0,
                sleep: Duration::from_secs(1),
            };

            let task = action.schedule_at_fixed_rate(
                name,
                &TokioSpawn::current(),
                MakeTokioDelay,
                None,
                Duration::from_secs(2),
                false,
            );

            tokio::time::sleep(Duration::from_secs(10)).await;
            task.cancel();
            let _ = task.into_inner().await;
        });

        rt.block_on(async move {
            let name = "tick-at-fixed-rate-3/2";
            let action = TickAction {
                name: name.to_string(),
                count: 0,
                sleep: Duration::from_secs(3),
            };

            let task = action.schedule_at_fixed_rate(
                name,
                &TokioSpawn::current(),
                MakeTokioDelay,
                None,
                Duration::from_secs(2),
                false,
            );

            tokio::time::sleep(Duration::from_secs(10)).await;
            task.cancel();
            let _ = task.into_inner().await;
        });
    }
}
