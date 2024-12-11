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

mod result;
pub use result::*;

mod generic;
pub use generic::*;

#[cfg(any(feature = "tokio-time", feature = "tokio-spawn"))]
mod tokio;
#[cfg(any(feature = "tokio-time", feature = "tokio-spawn"))]
pub use tokio::*;

// Roughly 30 years from now.
fn far_future() -> Instant {
    // API does not provide a way to obtain max `Instant`
    // or convert specific date in the future to instant.
    // 1000 years overflows on macOS, 100 years overflows on FreeBSD.
    Instant::now() + Duration::from_secs(86400 * 365 * 30)
}

fn make_instant_from(now: Instant, dur: Duration) -> Instant {
    now.checked_add(dur).unwrap_or_else(far_future)
}

fn make_instant_from_now(dur: Duration) -> Instant {
    make_instant_from(Instant::now(), dur)
}

pub trait MakeDelay: Send + 'static {
    fn delay(&self, at: Instant) -> impl Future<Output = ()> + Send;
}

pub trait Task {
    fn cancel(&self);
}

pub trait Spawn {
    type Task: Task;

    fn spawn<F: Future<Output = ()> + Send + 'static>(&self, future: F) -> Self::Task;
}
