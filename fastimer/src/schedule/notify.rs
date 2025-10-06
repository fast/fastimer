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

use super::execute_or_shutdown;
use crate::MakeDelay;
use crate::Spawn;
use crate::debug;
use crate::info;

/// Repeatable action that can be scheduled by notifications.
///
/// See [`NotifyActionExt`] for scheduling methods.
pub trait NotifyAction: Send + 'static {
    /// The name of the trait.
    fn name(&self) -> &str;

    /// Run the action.
    fn run(&mut self) -> impl Future<Output = ()> + Send;

    /// Return a future that resolves when the action is notified to run again.
    fn notified(&mut self) -> impl Future<Output = ()> + Send;
}

/// An extension trait for [`NotifyAction`] that provides scheduling methods.
pub trait NotifyActionExt: NotifyAction {
    /// Creates and executes a repeatable action that becomes enabled first after the given
    /// `initial_delay`, and subsequently when it is notified.
    fn schedule_by_notify<Fut, S, D>(
        mut self,
        is_shutdown: Fut,
        spawn: &S,
        make_delay: D,
        initial_delay: Option<Duration>,
    ) where
        Self: Sized,
        Fut: Future<Output = ()> + Send + 'static,
        S: Spawn,
        D: MakeDelay + Send + 'static,
    {
        spawn.spawn(async move {
            info!(
                "start scheduled task {} with initial delay {:?}",
                self.name(),
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

                    if execute_or_shutdown(self.notified(), &mut is_shutdown)
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

impl<T: NotifyAction> NotifyActionExt for T {}
