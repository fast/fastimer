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

mod arbitrary;

use std::future::Future;
use std::ops::ControlFlow;

pub use arbitrary::*;

mod notify;
pub use notify::*;

mod simple;
pub use simple::*;

mod select;
use crate::schedule::select::Either;
use crate::schedule::select::select;

/// Returns [`ControlFlow::Break`] if the caller should shut down; otherwise,
/// returns [`ControlFlow::Continue`] with the result of the action run.
async fn execute_or_shutdown<S, F, O>(f: F, is_shutdown: S) -> ControlFlow<(), O>
where
    S: Future<Output = ()>,
    F: Future<Output = O>,
{
    match select(is_shutdown, f).await {
        Either::Left(()) => ControlFlow::Break(()),
        Either::Right(o) => ControlFlow::Continue(o),
    }
}
