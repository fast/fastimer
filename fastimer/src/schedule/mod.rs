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

//! Repeatable and cancellable actions.

use std::future::Future;

mod arbitrary;
pub use arbitrary::*;

mod simple;
pub use simple::*;

use crate::schedule::select::select;
use crate::schedule::select::Either;

mod select;

/// Base trait for shutdown-able scheduled actions.
pub trait BaseAction: Send + 'static {
    /// The name of the trait.
    fn name(&self) -> &str;

    /// Return a future that resolves when the action is shutdown.
    ///
    /// By default, this function returns a future that never resolves, i.e., the action will never
    /// be shutdown.
    fn is_shutdown(&self) -> impl Future<Output = ()> + Send {
        std::future::pending()
    }

    /// A teardown hook that is called when the action is shutdown.
    fn teardown(&mut self) {}
}

/// Returns `true` if the action is shutdown.
async fn shutdown_or_delay<A, D>(action: &mut A, delay: D) -> bool
where
    A: BaseAction,
    D: Future<Output = ()>,
{
    let is_shutdown = action.is_shutdown();
    match select(is_shutdown, delay).await {
        Either::Left(()) => {
            #[cfg(feature = "logging")]
            log::debug!("scheduled task {} is stopped", action.name());
            action.teardown();
            true
        }
        Either::Right(()) => false,
    }
}
