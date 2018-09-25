use crate::task::{panic_local_waker_ref, panic_spawner_mut};
use crate::task::{noop_local_waker_ref, noop_spawner_mut};
use futures_core::task::Context;

/// Create a new [`task::Context`](futures_core::task::Context) where both
/// the [waker](futures_core::task::Context::waker) and
/// [spawner](futures_core::task::Context::spawner) will panic if used.
///
/// # Examples
///
/// ```should_panic
/// #![feature(futures_api)]
/// use futures_test::task;
///
/// let lw = task::panic_context();
/// lw.waker().wake(); // Will panic
/// ```
pub fn panic_context() -> Context<'static> {
    Context::new(panic_local_waker_ref(), panic_spawner_mut())
}

/// Create a new [`task::Context`](futures_core::task::Context) where the
/// [waker](futures_core::task::Context::waker) will ignore any calls to
/// `wake` while the [spawner](futures_core::task::Context::spawner) will
/// panic if used.
///
/// # Examples
///
/// ```
/// #![feature(async_await, futures_api, pin)]
/// use futures::future::Future;
/// use futures::task::Poll;
/// use futures_test::task::no_spawn_context;
/// use pin_utils::pin_mut;
///
/// let mut future = async { 5 };
/// pin_mut!(future);
///
/// assert_eq!(future.poll(&mut no_spawn_context()), Poll::Ready(5));
/// ```
pub fn no_spawn_context() -> Context<'static> {
    Context::new(noop_local_waker_ref(), panic_spawner_mut())
}

/// Create a new [`task::Context`](futures_core::task::Context) where the
/// [waker](futures_core::task::Context::waker) and
/// [spawner](futures_core::task::Context::spawner) will both ignore any
/// uses.
///
/// # Examples
///
/// ```
/// #![feature(async_await, futures_api, pin)]
/// use futures::future::Future;
/// use futures::task::Poll;
/// use futures_test::task::noop_context;
/// use pin_utils::pin_mut;
///
/// let mut future = async { 5 };
/// pin_mut!(future);
///
/// assert_eq!(future.poll(&mut noop_context()), Poll::Ready(5));
/// ```
pub fn noop_context() -> Context<'static> {
    Context::new(noop_local_waker_ref(), noop_spawner_mut())
}
