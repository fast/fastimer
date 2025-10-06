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

//! Tests for `Interval` with `tokio` runtime.

use std::time::Duration;
use std::time::Instant;

use fastimer::Interval;
use fastimer::MakeDelay;
use fastimer::MakeDelayExt;
use fastimer_tokio::MakeTokioDelay;

#[track_caller]
fn assert_duration_eq(actual: Duration, expected: Duration) {
    if expected.abs_diff(actual) > Duration::from_millis(250) {
        panic!("expected: {expected:?}, actual: {actual:?}");
    }
}

async fn assert_tick_about<D: MakeDelay>(interval: &mut Interval<D>, expected: Duration) {
    let start = Instant::now();
    interval.tick().await;
    let elapsed = start.elapsed();
    assert_duration_eq(elapsed, expected);
}

#[tokio::test]
async fn test_interval_ticks() {
    let mut interval = MakeTokioDelay::default().interval(Duration::from_secs(1));
    assert_tick_about(&mut interval, Duration::ZERO).await;

    for _ in 0..5 {
        assert_tick_about(&mut interval, Duration::from_secs(1)).await;
    }
}

#[tokio::test]
async fn test_interval_at_ticks() {
    let first_tick = Instant::now() + Duration::from_secs(2);

    let mut interval = MakeTokioDelay::default().interval_at(first_tick, Duration::from_secs(1));
    assert_tick_about(&mut interval, Duration::from_secs(2)).await;

    for _ in 0..5 {
        assert_tick_about(&mut interval, Duration::from_secs(1)).await;
    }
}
