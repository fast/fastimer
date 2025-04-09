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

use crate::MakeDelay;
use crate::Spawn;
use crate::debug;
use crate::info;
use crate::schedule::BaseAction;
use crate::schedule::execute_or_shutdown;
use crate::schedule::initial_delay_or_shutdown;

/// Repeatable action that can be scheduled by notifications.
///
/// See [`NotifyActionExt`] for scheduling methods.
pub trait NotifyAction: BaseAction {
    /// Return a future that resolves when the action is notified.
    ///
    /// The future should return `true` if the action should be stopped, and `false` if the action
    /// should be rescheduled.
    ///
    /// By default, this function calls [`is_shutdown`] to exit the action, and thus never
    /// reschedule the action. Implementations can override this method to provide custom
    /// notification logic, while still selects on [`is_shutdown`] to allow exiting the action.
    ///
    /// [`is_shutdown`]: BaseAction::is_shutdown
    fn notified(&mut self) -> impl Future<Output = bool> + Send {
        async move {
            self.is_shutdown().await;
            true
        }
    }
}

/// An extension trait for [`NotifyAction`] that provides scheduling methods.
pub trait NotifyActionExt: NotifyAction {
    /// Creates and executes a repeatable action that becomes enabled first after the given
    /// `initial_delay`, and subsequently when it is notified.
    fn schedule_by_notify<S, D>(mut self, spawn: &S, make_delay: D, initial_delay: Option<Duration>)
    where
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
                if initial_delay_or_shutdown(&mut self, make_delay, initial_delay).await {
                    break 'schedule;
                }

                loop {
                    debug!("executing scheduled task {}", self.name());

                    if execute_or_shutdown(&mut self).await {
                        break;
                    }

                    if self.notified().await {
                        break;
                    }
                }
            }

            info!("scheduled task {} is shutdown", self.name());
        });
    }
}

impl<T: NotifyAction> NotifyActionExt for T {}
