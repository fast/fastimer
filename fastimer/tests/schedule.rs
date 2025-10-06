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

//! Tests for `schedule` module with `tokio` runtime.

use std::time::Duration;
use std::time::Instant;

use fastimer::MakeDelay;
use fastimer::schedule::SimpleAction;
use fastimer::schedule::SimpleActionExt;
use fastimer_tokio::MakeTokioDelay;
use fastimer_tokio::TokioSpawn;
use logforth::append;

struct MySimpleAction {
    name: &'static str,
    counter: u8,
}

impl MySimpleAction {
    fn new(name: &'static str) -> Self {
        Self { name, counter: 0 }
    }
}

impl SimpleAction for MySimpleAction {
    fn name(&self) -> &str {
        self.name
    }

    async fn run(&mut self) {
        log::info!("[{}] starting turn {}", self.name, self.counter);
        MakeTokioDelay::default()
            .delay(Duration::from_secs(1))
            .await;
        self.counter += 1;
    }
}

#[tokio::test]
async fn test_simple_action() {
    logforth::starter_log::builder()
        .dispatch(|d| d.append(append::Stderr::default()))
        .apply();

    let spawn = TokioSpawn::default();
    let make_delay = MakeTokioDelay::default();

    let initial_delay = Some(Duration::from_secs(1));
    let shutdown = Instant::now() + Duration::from_secs(10);

    MySimpleAction::new("schedule_with_fixed_delay").schedule_with_fixed_delay(
        make_delay.delay_until(shutdown),
        &spawn,
        make_delay,
        initial_delay,
        Duration::from_secs(2),
    );

    MySimpleAction::new("schedule_at_fixed_rate").schedule_at_fixed_rate(
        make_delay.delay_until(shutdown),
        &spawn,
        make_delay,
        initial_delay,
        Duration::from_secs(2),
    );

    make_delay
        .delay_until(shutdown + Duration::from_secs(1))
        .await;
}
