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

use fastimer::make_instant_from_now;
use fastimer_driver::binary_heap_driver;

fn assert_duration_eq(actual: Duration, expected: Duration) {
    if expected.abs_diff(actual) > Duration::from_millis(5) {
        panic!("expected: {:?}, actual: {:?}", expected, actual);
    }
}

#[test]
fn test_binary_heap_driver() {
    let (mut driver, context, shutdown) = binary_heap_driver();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || loop {
        if driver.turn().is_break() {
            tx.send(()).unwrap();
            break;
        }
    });

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async move {
        let now = Instant::now();

        context.delay(Duration::from_secs(2)).await;
        assert_duration_eq(now.elapsed(), Duration::from_secs(2));

        let future = make_instant_from_now(Duration::from_secs(3));
        let f1 = context.delay_until(future);
        let f2 = context.delay_until(future);
        tokio::join!(f1, f2);
        assert_duration_eq(now.elapsed(), Duration::from_secs(3));

        shutdown.shutdown();
    });
    rx.recv_timeout(Duration::from_secs(1)).unwrap();
}
