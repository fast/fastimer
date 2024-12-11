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

fn main() {
    let (mut driver, context, shutdown) = fastimer::driver::driver();

    std::thread::spawn(move || loop {
        if driver.turn() {
            break;
        }
    });

    let delay = context.delay(std::time::Duration::from_secs(1));
    pollster::block_on(delay); // finish after 1 second
    shutdown.shutdown();
}
