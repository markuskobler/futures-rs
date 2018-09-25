use core::marker::Unpin;
use core::pin::Pin;
use futures_core::future::{Future, TryFuture};
use futures_core::task::{LocalWaker, Poll};
use pin_utils::{unsafe_pinned, unsafe_unpinned};

/// Future for the [`map_ok`](super::TryFutureExt::map_ok) combinator.
#[derive(Debug)]
#[must_use = "futures do nothing unless polled"]
pub struct MapOk<Fut, F> {
    future: Fut,
    f: Option<F>,
}

impl<Fut, F> MapOk<Fut, F> {
    unsafe_pinned!(future: Fut);
    unsafe_unpinned!(f: Option<F>);

    /// Creates a new MapOk.
    pub(super) fn new(future: Fut, f: F) -> MapOk<Fut, F> {
        MapOk { future, f: Some(f) }
    }
}

impl<Fut: Unpin, F> Unpin for MapOk<Fut, F> {}

impl<Fut, F, T> Future for MapOk<Fut, F>
    where Fut: TryFuture,
          F: FnOnce(Fut::Ok) -> T,
{
    type Output = Result<T, Fut::Error>;

    fn poll(
        mut self: Pin<&mut Self>,
        lw: &LocalWaker,
    ) -> Poll<Self::Output> {
        match self.future().try_poll(lw) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => {
                let op = self.f().take()
                    .expect("MapOk must not be polled after it returned `Poll::Ready`");
                Poll::Ready(result.map(op))
            }
        }
    }
}
