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

use fastimer::tokio::MakeTokioDelay;
use mea::latch::Latch;
use mea::waitgroup::WaitGroup;

#[derive(Debug, Clone)]
pub struct Shutdown {
    latch: Arc<Latch>,
    wg: WaitGroup,
}

impl Shutdown {
    pub fn new() -> Self {
        Shutdown {
            latch: Arc::new(Latch::new(1)),
            wg: WaitGroup::new(),
        }
    }

    pub fn shutdown(&self) {
        self.latch.count_down();
    }

    pub async fn is_shutdown(&self) {
        self.latch.wait().await;
    }

    pub async fn await_shutdown(self) {
        self.wg.await;
    }
}

pub async fn timeout(fut: impl Future) {
    const T: Duration = Duration::from_secs(10);
    fastimer::timeout(T, fut, MakeTokioDelay).await.unwrap();
}
