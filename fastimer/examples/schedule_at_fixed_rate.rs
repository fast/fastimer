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

use fastimer::schedule::BaseAction;
use fastimer::schedule::SimpleAction;
use fastimer::schedule::SimpleActionExt;
use fastimer::tokio::MakeTokioDelay;
use fastimer::tokio::TokioSpawn;

use crate::common::Shutdown;

mod common;

#[derive(Debug)]
struct TickAction {
    name: String,
    count: u32,
    sleep: Duration,
    shutdown: Shutdown,
}

impl BaseAction for TickAction {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_shutdown(&self) -> impl Future<Output = ()> + Send {
        self.shutdown.is_shutdown()
    }
}

impl SimpleAction for TickAction {
    async fn run(&mut self) {
        self.count += 1;
        log::info!("[{}] tick start: {}", self.name(), self.count);
        tokio::time::sleep(self.sleep).await;
        log::info!("[{}] tick end: {}", self.name(), self.count);
    }
}

async fn do_schedule_at_fixed_rate(sleep: u64, period: u64) {
    let shutdown = Shutdown::new();
    let tick = TickAction {
        name: format!("fixed-rate-{sleep}/{period}"),
        count: 0,
        sleep: Duration::from_secs(sleep),
        shutdown: shutdown.clone(),
    };
    tick.schedule_at_fixed_rate(
        &TokioSpawn::current(),
        MakeTokioDelay,
        None,
        Duration::from_secs(period),
    );
    tokio::time::sleep(Duration::from_secs(10)).await;
    shutdown.shutdown();
    common::timeout(shutdown.await_shutdown()).await;
}

fn main() {
    logforth::stderr().apply();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(do_schedule_at_fixed_rate(1, 2));
    rt.block_on(do_schedule_at_fixed_rate(3, 2));
    rt.block_on(do_schedule_at_fixed_rate(5, 2));
}
