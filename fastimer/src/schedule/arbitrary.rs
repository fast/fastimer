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
use std::ops::ControlFlow;
use std::time::Duration;
use std::time::Instant;

use crate::MakeDelay;
use crate::Spawn;
use crate::debug;
use crate::info;
use crate::schedule::execute_or_shutdown;

/// Repeatable action that can be scheduled with arbitrary delay.
///
/// See [`ArbitraryDelayActionExt`] for scheduling methods.
pub trait ArbitraryDelayAction: Send + 'static {
    /// The name of the trait.
    fn name(&self) -> &str;

    /// Run the action.
    ///
    /// Return an Instant that indicates when to schedule the next run.
    fn run(&mut self) -> impl Future<Output = Instant> + Send;
}

/// An extension trait for [`ArbitraryDelayAction`] that provides scheduling methods.
pub trait ArbitraryDelayActionExt: ArbitraryDelayAction {
    /// Creates and executes a repeatable action that becomes enabled first after the given
    /// `initial_delay`, and subsequently based on the result of the action.
    fn schedule_with_arbitrary_delay<F, Fut, S, D>(
        mut self,
        mut is_shutdown: F,
        spawn: &S,
        make_delay: D,
        initial_delay: Option<Duration>,
    ) where
        Self: Sized,
        F: FnMut() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send,
        S: Spawn,
        D: MakeDelay + Send + 'static,
    {
        spawn.spawn(async move {
            info!(
                "start scheduled task {} with initial delay {:?}",
                self.name(),
                initial_delay
            );

            'schedule: {
                if let Some(initial_delay) = initial_delay {
                    if initial_delay > Duration::ZERO
                        && execute_or_shutdown(make_delay.delay(initial_delay), is_shutdown())
                            .await
                            .is_break()
                    {
                        break 'schedule;
                    }
                }

                loop {
                    debug!("executing scheduled task {}", self.name());

                    let next = match execute_or_shutdown(self.run(), is_shutdown()).await {
                        ControlFlow::Continue(next) => next,
                        ControlFlow::Break(()) => break,
                    };

                    if execute_or_shutdown(make_delay.delay_util(next), is_shutdown())
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

impl<T: ArbitraryDelayAction> ArbitraryDelayActionExt for T {}
