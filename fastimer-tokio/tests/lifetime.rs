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

use fastimer_tokio::MakeTokioDelay;

fn timer() -> &'static MakeTokioDelay {
    static TIMER: MakeTokioDelay = MakeTokioDelay;
    &TIMER
}

struct TimerWrapper {
    timer: &'static MakeTokioDelay,
}

fn trick_timer() -> TimerWrapper {
    TimerWrapper { timer: timer() }
}

impl fastimer::MakeDelay for TimerWrapper {
    type Delay = tokio::time::Sleep;

    fn delay_util(&self, at: Instant) -> Self::Delay {
        self.timer.delay_util(at)
    }

    fn delay(&self, duration: Duration) -> Self::Delay {
        self.timer.delay(duration)
    }
}

#[test]
fn test_lifetime_ok() {
    let rt1 = tokio::runtime::Runtime::new().unwrap();
    let rt2 = tokio::runtime::Runtime::new().unwrap();

    let spawn = rt1.spawn(async move {
        let mut interval = fastimer::interval(Duration::from_secs(1), trick_timer());
        for _ in 0..3 {
            interval.tick().await;
        }
    });

    rt2.block_on(spawn).unwrap();
}

// #[test]
// fn test_lifetime_nok() {
//     let rt1 = tokio::runtime::Runtime::new().unwrap();
//     let rt2 = tokio::runtime::Runtime::new().unwrap();
//
//     let spawn = rt1.spawn(async move {
//         let mut interval = fastimer::interval(Duration::from_secs(1), timer());
//         for _ in 0..3 {
//             interval.tick().await;
//         }
//     });
//
//     rt2.block_on(spawn).unwrap();
// }
