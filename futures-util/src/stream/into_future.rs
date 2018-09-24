use core::marker::Unpin;
use core::pin::Pin;
use futures_core::future::Future;
use futures_core::stream::Stream;
use futures_core::task::{self, Poll};

/// A combinator used to temporarily convert a stream into a future.
///
/// This future is returned by the `Stream::into_future` method.
#[derive(Debug)]
#[must_use = "futures do nothing unless polled"]
pub struct StreamFuture<St> {
    stream: Option<St>,
}

impl<St: Stream + Unpin> Unpin for StreamFuture<St> {}

impl<St: Stream + Unpin> StreamFuture<St> {
    pub(super) fn new(stream: St) -> StreamFuture<St> {
        StreamFuture { stream: Some(stream) }
    }
    /// Acquires a reference to the underlying stream that this combinator is
    /// pulling from.
    ///
    /// This method returns an `Option` to account for the fact that `StreamFuture`'s
    /// implementation of `Future::poll` consumes the underlying stream during polling
    /// in order to return it to the caller of `Future::poll` if the stream yielded
    /// an element.
    pub fn get_ref(&self) -> Option<&St> {
        self.stream.as_ref()
    }

    /// Acquires a mutable reference to the underlying stream that this
    /// combinator is pulling from.
    ///
    /// Note that care must be taken to avoid tampering with the state of the
    /// stream which may otherwise confuse this combinator.
    ///
    /// This method returns an `Option` to account for the fact that `StreamFuture`'s
    /// implementation of `Future::poll` consumes the underlying stream during polling
    /// in order to return it to the caller of `Future::poll` if the stream yielded
    /// an element.
    pub fn get_mut(&mut self) -> Option<&mut St> {
        self.stream.as_mut()
    }

    /// Consumes this combinator, returning the underlying stream.
    ///
    /// Note that this may discard intermediate state of this combinator, so
    /// care should be taken to avoid losing resources when this is called.
    ///
    /// This method returns an `Option` to account for the fact that `StreamFuture`'s
    /// implementation of `Future::poll` consumes the underlying stream during polling
    /// in order to return it to the caller of `Future::poll` if the stream yielded
    /// an element.
    pub fn into_inner(self) -> Option<St> {
        self.stream
    }
}

impl<St: Stream + Unpin> Future for StreamFuture<St> {
    type Output = (Option<St::Item>, St);

    fn poll(
        mut self: Pin<&mut Self>,
        lw: &LocalWaker
    ) -> Poll<Self::Output> {
        let item = {
            let s = self.stream.as_mut().expect("polling StreamFuture twice");
            ready!(Pin::new(s).poll_next(cx))
        };
        let stream = self.stream.take().unwrap();
        Poll::Ready((item, stream))
    }
}
