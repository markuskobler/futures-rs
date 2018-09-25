//! Definition of the `PollFn` adapter combinator

use core::marker::Unpin;
use core::pin::Pin;
use futures_core::future::Future;
use futures_core::task::{LocalWaker, Poll};

/// A future which wraps a function returning [`Poll`].
///
/// Created by the [`poll_fn()`] function.
#[derive(Debug)]
#[must_use = "futures do nothing unless polled"]
pub struct PollFn<F> {
    f: F,
}

impl<F> Unpin for PollFn<F> {}

/// Creates a new future wrapping around a function returning [`Poll`].
///
/// Polling the returned future delegates to the wrapped function.
///
/// # Examples
///
/// ```
/// #![feature(async_await, await_macro, futures_api)]
/// # futures::executor::block_on(async {
/// use futures::future::poll_fn;
/// use futures::task::{LocalWaker, Poll};
///
/// fn read_line(lw: &LocalWaker) -> Poll<String> {
///     Poll::Ready("Hello, World!".into())
/// }
///
/// let read_future = poll_fn(read_line);
/// assert_eq!(await!(read_future), "Hello, World!".to_owned());
/// # });
/// ```
pub fn poll_fn<T, F>(f: F) -> PollFn<F>
where
    F: FnMut(&LocalWaker) -> Poll<T>
{
    PollFn { f }
}

impl<T, F> Future for PollFn<F>
    where F: FnMut(&LocalWaker) -> Poll<T>,
{
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, lw: &LocalWaker) -> Poll<T> {
        (&mut self.f)(lw)
    }
}
