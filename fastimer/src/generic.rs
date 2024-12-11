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

use crate::make_instant_from_now;
use crate::MakeDelay;
use crate::Spawn;

/// The result of a scheduled action, that indicates whether the action should be continued or
/// stopped.
pub enum ScheduleOp {
    /// Run the action again at the specified instant.
    Continue(Instant),
    /// Stop the action.
    Break,
}

/// A generic action that can be scheduled.
pub trait GenericAction: Send + 'static {
    /// The name of the action.
    fn name(&self) -> &str;

    /// Run the action.
    fn run(&mut self) -> impl Future<Output = ScheduleOp> + Send;
}

/// An extension trait for [`GenericAction`] that provides scheduling methods.
pub trait GenericActionExt: GenericAction {
    /// Creates and executes a repeatable action that becomes enabled first after the given
    /// `initial_delay`, and subsequently based on the result of the action.
    ///
    /// If the action returns [`ScheduleOp::Continue`], the action will be scheduled again at the
    /// specified instant. If the action returns [`ScheduleOp::Break`], the action will be stopped.
    fn schedule<S, D>(
        mut self,
        spawn: &S,
        make_delay: D,
        initial_delay: Option<Duration>,
    ) -> S::Task
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
                    make_delay.delay(make_instant_from_now(initial_delay)).await;
                }
            }

            loop {
                #[cfg(feature = "logging")]
                log::debug!("executing scheduled task {}", self.name());

                match self.run().await {
                    ScheduleOp::Break => {
                        #[cfg(feature = "logging")]
                        log::debug!("scheduled task {} is stopped", self.name());
                        break;
                    }
                    ScheduleOp::Continue(at) => {
                        make_delay.delay(at).await;
                    }
                }
            }
        })
    }
}

impl<T: GenericAction> GenericActionExt for T {}
