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

use crate::schedule::shutdown_or_delay;
use crate::schedule::BaseAction;
use crate::MakeDelay;
use crate::Spawn;

/// Repeatable action that can be scheduled with arbitrary delay.
pub trait ArbitraryDelayAction: BaseAction {
    /// Run the action.
    ///
    /// Return an Instant that indicates when to schedule the next run.
    fn run(&mut self) -> impl Future<Output = Instant> + Send;
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

            if_chain::if_chain! {
                if let Some(initial_delay) = initial_delay;
                if initial_delay > Duration::ZERO;
                if shutdown_or_delay(&mut self, make_delay.delay(initial_delay)).await;
                then { return; }
            }

            loop {
                #[cfg(feature = "logging")]
                log::debug!("executing scheduled task {}", self.name());
                let next = self.run().await;

                if shutdown_or_delay(&mut self, make_delay.delay_util(next)).await {
                    return;
                }
            }
        });
    }
}

impl<T: ArbitraryDelayAction> ArbitraryDelayActionExt for T {}
