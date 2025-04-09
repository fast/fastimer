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

use std::time::Duration;
use std::time::Instant;

use crate::MakeDelay;
use crate::Spawn;
use crate::debug;
use crate::info;
use crate::schedule::BaseAction;
use crate::schedule::delay_or_shutdown;
use crate::schedule::execute_or_shutdown;

/// Repeatable action that can be scheduled with arbitrary delay.
///
/// See [`ArbitraryDelayActionExt`] for scheduling methods.
pub trait ArbitraryDelayAction: BaseAction {
    /// Return an Instant that indicates when to schedule the next run.
    fn next_run_at(&self) -> Instant;
}

/// An extension trait for [`ArbitraryDelayAction`] that provides scheduling methods.
pub trait ArbitraryDelayActionExt: ArbitraryDelayAction {
    /// Creates and executes a repeatable action that becomes enabled first after the given
    /// `initial_delay`, and subsequently based on the result of the action.
    fn schedule_with_arbitrary_delay<S, D>(
        mut self,
        spawn: &S,
        make_delay: D,
        initial_delay: Option<Duration>,
    ) where
        Self: Sized,
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
                        && delay_or_shutdown(&mut self, make_delay.delay(initial_delay)).await
                    {
                        break 'schedule;
                    }
                }

                loop {
                    debug!("executing scheduled task {}", self.name());

                    if execute_or_shutdown(&mut self).await {
                        break;
                    }

                    let next = self.next_run_at();
                    if delay_or_shutdown(&mut self, make_delay.delay_util(next)).await {
                        break;
                    }
                }
            }

            info!("scheduled task {} is shutdown", self.name());
        });
    }
}

impl<T: ArbitraryDelayAction> ArbitraryDelayActionExt for T {}
