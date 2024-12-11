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
use std::sync::Arc;
use std::time::Duration;

use fastimer::schedule::SimpleAction;
use fastimer::schedule::SimpleActionExt;
use fastimer::tokio::MakeTokioDelay;
use fastimer::tokio::TokioSpawn;
use mea::latch::Latch;
use mea::waitgroup::WaitGroup;

struct TickAction {
    count: u32,

    latch: Arc<Latch>,
    _wg: WaitGroup,
}

impl SimpleAction for TickAction {
    fn name(&self) -> &str {
        "tick"
    }

    async fn run(&mut self) {
        self.count += 1;
        println!("tick: {}", self.count);
    }

    fn is_shutdown(&self) -> impl Future<Output = ()> + Send {
        self.latch.wait()
    }
}

fn main() {
    let wg = WaitGroup::new();
    let latch = Arc::new(Latch::new(1));

    let tick = TickAction {
        count: 0,
        latch: latch.clone(),
        _wg: wg.clone(),
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async move {
        tick.schedule_with_fixed_delay(
            &TokioSpawn::current(),
            MakeTokioDelay,
            None,
            Duration::from_secs(1),
        );

        tokio::time::sleep(Duration::from_secs(5)).await;
        latch.count_down();
        fastimer::timeout(Duration::from_secs(5), wg, MakeTokioDelay)
            .await
            .unwrap();
    });
}
