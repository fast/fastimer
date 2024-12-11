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

use crate::schedule::select::select;
use crate::schedule::select::Either;
use crate::MakeDelay;
use crate::Spawn;

/// A generic action that can be scheduled.
pub trait ArbitraryDelayAction: Send + 'static {
    /// The name of the action.
    fn name(&self) -> &str;

    /// Run the action.
    ///
    /// Return an Instant that indicates when to schedule the next run.
    fn run(&mut self) -> impl Future<Output = Instant> + Send;

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

/// An extension trait for [`ArbitraryDelayAction`] that provides scheduling methods.
pub trait ArbitraryDelayActionExt: ArbitraryDelayAction {
    /// Creates and executes a repeatable action that becomes enabled first after the given
    /// `initial_delay`, and subsequently based on the result of the action.
    fn schedule<S, D>(mut self, spawn: &S, make_delay: D, initial_delay: Option<Duration>)
    where
        Self: Sized,
        S: Spawn,
        D: MakeDelay,
    {
        spawn.spawn(async move {
            #[cfg(feature = "logging")]
            log::debug!(
                "start scheduled task {} with initial delay {:?}",
                self.name(),
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

                let next = self.run().await;
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

impl<T: ArbitraryDelayAction> ArbitraryDelayActionExt for T {}

#[cfg(all(test, feature = "test"))]
mod tests {
    use std::future::Future;
    use std::sync::Arc;
    use std::time::Duration;
    use std::time::Instant;

    use mea::latch::Latch;
    use mea::waitgroup::WaitGroup;

    use crate::schedule::ArbitraryDelayAction;
    use crate::schedule::ArbitraryDelayActionExt;
    use crate::timeout;
    use crate::tokio::MakeTokioDelay;
    use crate::tokio::TokioSpawn;

    struct TickAction {
        name: String,
        count: u32,

        latch: Arc<Latch>,
        _wg: WaitGroup,
    }

    impl ArbitraryDelayAction for TickAction {
        fn name(&self) -> &str {
            &self.name
        }

        async fn run(&mut self) -> Instant {
            self.count += 1;
            log::info!("[{}] tick count: {}", self.name, self.count);
            Instant::now() + Duration::from_secs(self.count as u64)
        }

        fn is_shutdown(&self) -> impl Future<Output = ()> + Send {
            self.latch.wait()
        }
    }

    #[test]
    fn test_schedule() {
        let _ = logforth::stderr().try_apply();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let wg = WaitGroup::new();
        let latch = Arc::new(Latch::new(1));

        rt.block_on(async move {
            let action = TickAction {
                name: "fixed-delay".to_string(),
                count: 0,
                latch: latch.clone(),
                _wg: wg.clone(),
            };

            action.schedule(&TokioSpawn::current(), MakeTokioDelay, None);

            tokio::time::sleep(Duration::from_secs(10)).await;
            latch.count_down();
            timeout(Duration::from_secs(5), wg, MakeTokioDelay)
                .await
                .unwrap();
        });
    }
}
