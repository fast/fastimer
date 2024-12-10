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

#[cfg(feature = "tokio-time")]
pub use delay::*;

#[cfg(feature = "tokio-time")]
mod delay {
    use std::future::Future;
    use std::time::Instant;

    use crate::MakeDelay;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct MakeTokioDelay;

    impl MakeDelay for MakeTokioDelay {
        fn delay(&self, at: Instant) -> impl Future<Output = ()> {
            tokio::time::sleep_until(tokio::time::Instant::from_std(at))
        }
    }
}

#[cfg(feature = "tokio-spawn")]
pub use spawn::*;

#[cfg(feature = "tokio-spawn")]
mod spawn {
    use std::future::Future;

    use crate::Spawn;
    use crate::Task;

    #[derive(Clone, Debug, Default)]
    pub struct TokioSpawn(Option<tokio::runtime::Handle>);

    impl TokioSpawn {
        pub fn with_handle(mut self, handle: tokio::runtime::Handle) -> Self {
            self.0 = Some(handle);
            self
        }

        pub fn current() -> Self {
            Self::default().with_handle(tokio::runtime::Handle::current())
        }
    }

    #[derive(Debug)]
    pub struct TokioTask(tokio::task::JoinHandle<()>);

    impl TokioTask {
        pub fn into_inner(self) -> tokio::task::JoinHandle<()> {
            self.0
        }
    }

    impl Task for TokioTask {
        fn cancel(&self) {
            self.0.abort();
        }
    }

    impl Spawn for TokioSpawn {
        type Task = TokioTask;

        fn spawn<F: Future<Output = ()> + Send + 'static>(&self, future: F) -> Self::Task {
            TokioTask(match &self.0 {
                None => tokio::spawn(future),
                Some(handle) => handle.spawn(future),
            })
        }
    }
}
