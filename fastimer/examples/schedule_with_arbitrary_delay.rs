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

use fastimer::make_instant_from_now;
use fastimer::schedule::ArbitraryDelayAction;
use fastimer::schedule::ArbitraryDelayActionExt;
use fastimer::schedule::BaseAction;
use fastimer::tokio::MakeTokioDelay;
use fastimer::tokio::TokioSpawn;

use crate::common::Shutdown;

mod common;

#[derive(Debug)]
struct TickAction {
    count: u32,
    shutdown: Shutdown,
}

impl BaseAction for TickAction {
    fn name(&self) -> &str {
        "tick-arbitrary"
    }

    fn is_shutdown(&self) -> impl Future<Output = ()> + Send {
        self.shutdown.is_shutdown()
    }
}

impl ArbitraryDelayAction for TickAction {
    async fn run(&mut self) -> Instant {
        self.count += 1;
        log::info!("tick: {}", self.count);
        make_instant_from_now(Duration::from_secs(self.count as u64))
    }
}

fn main() {
    logforth::stderr().apply();

    let shutdown = Shutdown::new();
    let tick = TickAction {
        count: 0,
        shutdown: shutdown.clone(),
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async move {
        tick.schedule_with_arbitrary_delay(
            &TokioSpawn::current(),
            MakeTokioDelay,
            Some(Duration::from_secs(1)),
        );

        tokio::time::sleep(Duration::from_secs(10)).await;
        shutdown.shutdown();
        common::timeout(shutdown.await_shutdown()).await;
    });
}
