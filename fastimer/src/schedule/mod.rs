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

use self::select::Either;
use self::select::select;

mod arbitrary;
pub use arbitrary::*;

mod notify;
pub use notify::*;

mod simple;
pub use simple::*;

mod select;

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
}

/// Returns `true` if the action is shutdown.
async fn delay_or_shutdown<A, D>(action: &mut A, delay: D) -> bool
where
    A: BaseAction,
    D: Future<Output = ()>,
{
    let is_shutdown = action.is_shutdown();
    match select(is_shutdown, delay).await {
        Either::Left(()) => true,
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
        Either::Left(()) => true,
        Either::Right(()) => false,
    }
}
