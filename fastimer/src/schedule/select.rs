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
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

#[derive(Debug, Clone)]
pub enum Either<A, B> {
    Left(A),
    Right(B),
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
#[derive(Debug)]
#[pin_project::pin_project]
pub struct Select<A, B> {
    #[pin]
    a: A,
    #[pin]
    b: B,
}

impl<A, B> Future for Select<A, B>
where
    A: Future,
    B: Future,
{
    type Output = Either<A::Output, B::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        if let Poll::Ready(a) = this.a.poll(cx) {
            return Poll::Ready(Either::Left(a));
        }
        if let Poll::Ready(b) = this.b.poll(cx) {
            return Poll::Ready(Either::Right(b));
        }
        Poll::Pending
    }
}

pub fn select<A, B>(a: A, b: B) -> Select<A, B>
where
    A: Future,
    B: Future,
{
    Select { a, b }
}
