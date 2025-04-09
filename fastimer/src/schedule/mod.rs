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
use std::time::Duration;

mod arbitrary;
pub use arbitrary::*;

mod notify;
pub use notify::*;

mod simple;
pub use simple::*;

use crate::MakeDelay;
use crate::info;

mod select;

use crate::schedule::select::Either;
use crate::schedule::select::select;

/// Base trait for shutdown-able scheduled actions.
pub trait BaseAction: Send + 'static {
    /// The name of the trait.
    fn name(&self) -> &str;

    /// Return a future that resolves when the action is shutdown.
    ///
    /// By default, this function returns a future that never resolves, i.e., the action will never
    /// be shutdown.
    fn is_shutdown(&self) -> impl Future<Output = ()> + Send + 'static {
        std::future::pending()
    }

    /// Run the action.
    fn run(&mut self) -> impl Future<Output = ()> + Send;

    /// A teardown hook that is called when the action is shutdown.
    fn teardown(&mut self) {}
}

/// Returns `None` if the action is shutdown; otherwise, returns `Some(make_delay)`
/// to give back the `make_delay` for further scheduling.
async fn initial_delay_or_shutdown<A, D>(
    action: &mut A,
    make_delay: D,
    initial_delay: Option<Duration>,
) -> Option<D>
where
    A: BaseAction,
    D: MakeDelay,
{
    let Some(initial_delay) = initial_delay else {
        return Some(make_delay);
    };

    if initial_delay.is_zero() {
        return Some(make_delay);
    }

    if delay_or_shutdown(action, make_delay.delay(initial_delay)).await {
        None
    } else {
        Some(make_delay)
    }
}

/// Returns `true` if the action is shutdown.
async fn delay_or_shutdown<A, D>(action: &mut A, delay: D) -> bool
where
    A: BaseAction,
    D: Future<Output = ()>,
{
    let is_shutdown = action.is_shutdown();
    match select(is_shutdown, delay).await {
        Either::Left(()) => {
            info!("scheduled task {} is stopped", action.name());
            action.teardown();
            true
        }
        Either::Right(()) => false,
    }
}

/// Returns `true` if the action is shutdown.
async fn execute_or_shutdown<A>(action: &mut A) -> bool
where
    A: BaseAction,
{
    let is_shutdown = action.is_shutdown();
    let execute = action.run();
    match select(is_shutdown, execute).await {
        Either::Left(()) => {
            info!("scheduled task {} is stopped", action.name());
            action.teardown();
            true
        }
        Either::Right(()) => false,
    }
}
