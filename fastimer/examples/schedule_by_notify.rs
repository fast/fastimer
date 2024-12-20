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

use fastimer::schedule::BaseAction;
use fastimer::schedule::NotifyAction;
use fastimer::schedule::NotifyActionExt;
use fastimer::tokio::MakeTokioDelay;
use fastimer::tokio::TokioSpawn;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;

use crate::common::Shutdown;

mod common;

#[derive(Debug)]
struct TickAction {
    count: u32,
    shutdown: Shutdown,

    #[expect(unused)]
    tx: UnboundedSender<()>,
    rx: UnboundedReceiver<()>,
}

impl BaseAction for TickAction {
    fn name(&self) -> &str {
        "tick-notify"
    }

    fn is_shutdown(&self) -> impl Future<Output = ()> + Send {
        self.shutdown.is_shutdown()
    }
}

impl NotifyAction for TickAction {
    async fn run(&mut self) {
        self.count += 1;
        log::info!("tick: {}", self.count);
    }

    async fn notified(&mut self) -> bool {
        let timeout = tokio::time::sleep(Duration::from_secs(5));
        tokio::select! {
            _ = timeout => false,
            _ = self.rx.recv() => {
                while self.rx.try_recv().is_ok() {}
                false
            }
            _ = self.shutdown.is_shutdown() => true,
        }
    }
}

fn main() {
    logforth::stderr().apply();

    let shutdown = Shutdown::new();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let tick = TickAction {
        count: 0,
        shutdown: shutdown.clone(),
        tx: tx.clone(),
        rx,
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async move {
        tick.schedule_by_notify(&TokioSpawn::current(), MakeTokioDelay, None);

        tokio::time::sleep(Duration::from_secs(1)).await;
        tx.send(()).unwrap();
        tokio::time::sleep(Duration::from_secs(8)).await;
        tx.send(()).unwrap();
        tokio::time::sleep(Duration::from_secs(1)).await;
        shutdown.shutdown();
        common::timeout(shutdown.await_shutdown()).await;
    });
}
