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

pub enum ScheduleOp {
    Continue(Instant),
    Break,
}

pub trait GenericAction: Send + 'static {
    fn run(&mut self) -> impl Future<Output = ScheduleOp> + Send;
}

pub trait GenericActionExt: GenericAction {
    fn schedule<N, S, D>(
        mut self,
        name: N,
        spawn: &S,
        make_delay: D,
        initial_delay: Option<Duration>,
    ) -> S::Task
    where
        Self: Sized,
        N: Into<String>,
        S: Spawn,
        D: MakeDelay,
    {
        let name = name.into();
        spawn.spawn(async move {
            log::debug!("start scheduled task {name} with initial delay {initial_delay:?}");

            if let Some(initial_delay) = initial_delay {
                if initial_delay > Duration::ZERO {
                    make_delay.delay(make_instant_from_now(initial_delay)).await;
                }
            }

            loop {
                log::debug!("executing scheduled task {name}");

                match self.run().await {
                    ScheduleOp::Break => {
                        log::debug!("scheduled task {name} is stopped");
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
