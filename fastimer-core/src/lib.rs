//! # Fastimer Core APIs

use std::future::Future;
use std::time::Duration;
use std::time::Instant;

/// Create a far future instant.
pub fn far_future() -> Instant {
    // Roughly 30 years from now.
    // API does not provide a way to obtain max `Instant`
    // or convert specific date in the future to instant.
    // 1000 years overflows on macOS, 100 years overflows on FreeBSD.
    Instant::now() + Duration::from_secs(86400 * 365 * 30)
}

/// Create an instant from the given instant and a duration.
pub fn make_instant_from(now: Instant, dur: Duration) -> Instant {
    now.checked_add(dur).unwrap_or_else(far_future)
}

/// Create an instant from [`Instant::now`] and a duration.
pub fn make_instant_from_now(dur: Duration) -> Instant {
    make_instant_from(Instant::now(), dur)
}

/// A trait for creating delay futures.
///
/// See [`MakeDelayExt`] for extension methods.
pub trait MakeDelay {
    /// The future returned by the `delay`/`delay_until` method.
    type Delay: Future<Output = ()> + Send;

    /// Create a future that completes at the specified instant.
    fn delay_until(&self, at: Instant) -> Self::Delay;

    /// Create a future that completes after the specified duration.
    fn delay(&self, duration: Duration) -> Self::Delay {
        self.delay_until(make_instant_from_now(duration))
    }
}

/// A trait for spawning futures.
pub trait Spawn {
    /// Spawn a future and return a cancellable future.
    fn spawn<F: Future<Output = ()> + Send + 'static>(&self, future: F);
}
