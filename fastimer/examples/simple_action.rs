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

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Duration;

use fastimer::tokio::MakeTokioDelay;
use fastimer::tokio::TokioSpawn;
use fastimer::SimpleActionExt;

struct TickAction {
    count: u32,
    shutdown: Arc<AtomicBool>,
    tx_stopped: Sender<()>,
}

impl fastimer::SimpleAction for TickAction {
    fn name(&self) -> &str {
        "tick"
    }

    async fn run(&mut self) -> bool {
        if self.shutdown.load(Ordering::Acquire) {
            println!("shutdown");
            let _ = self.tx_stopped.send(());
            true
        } else {
            println!("tick: {}", self.count);
            self.count += 1;
            false
        }
    }
}

fn main() {
    let (tx, rx) = mpsc::channel();
    let shutdown = Arc::new(AtomicBool::new(false));

    let tick = TickAction {
        count: 0,
        shutdown: shutdown.clone(),
        tx_stopped: tx,
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
        shutdown.store(true, Ordering::Release);
    });
    let _ = rx.recv_timeout(Duration::from_secs(5));
}
