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

use std::sync::LazyLock;
use std::time::Duration;
use std::time::Instant;

use fastimer::driver::TimeContext;
use fastimer::make_instant_from_now;

fn static_context() -> &'static TimeContext {
    static CONTEXT: LazyLock<TimeContext> = LazyLock::new(|| {
        let (mut driver, context, _) = fastimer::driver::driver();
        std::thread::Builder::new()
            .name("fastimer-global".to_string())
            .spawn(move || loop {
                if driver.turn() {
                    break;
                }
            })
            .expect("cannot spawn fastimer-global thread");
        context
    });

    &CONTEXT
}

fn assert_duration_eq(actual: Duration, expected: Duration) {
    if expected.abs_diff(expected) > Duration::from_millis(5) {
        panic!("expected: {:?}, actual: {:?}", expected, actual);
    }
}

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async move {
        let now = Instant::now();

        static_context().delay(Duration::from_secs(2)).await;
        assert_duration_eq(now.elapsed(), Duration::from_secs(2));

        let future = make_instant_from_now(Duration::from_secs(3));
        let f1 = static_context().delay_until(future);
        let f2 = static_context().delay_until(future);
        tokio::join!(f1, f2);
        assert_duration_eq(now.elapsed(), Duration::from_secs(3));
    });
}
