//! Asynchronous streams.

use crate::task::{LocalWaker, Poll};
use core::marker::Unpin;
use core::ops;
use core::pin::Pin;

#[cfg(feature = "either")]
use either::Either;

mod stream_obj;
pub use self::stream_obj::{StreamObj,LocalStreamObj,UnsafeStreamObj};

/// A stream of values produced asynchronously.
///
/// If `Future<Output = T>` is an asynchronous version of `T`, then `Stream<Item
/// = T>` is an asynchronous version of `Iterator<Item = T>`. A stream
/// represents a sequence of value-producing events that occur asynchronously to
/// the caller.
///
/// The trait is modeled after `Future`, but allows `poll_next` to be called
/// even after a value has been produced, yielding `None` once the stream has
/// been fully exhausted.
pub trait Stream {
    /// Values yielded by the stream.
    type Item;

    /// Attempt to pull out the next value of this stream, registering the
    /// current task for wakeup if the value is not yet available, and returning
    /// `None` if the stream is exhausted.
    ///
    /// # Return value
    ///
    /// There are several possible return values, each indicating a distinct
    /// stream state:
    ///
    /// - `Poll::Pending` means that this stream's next value is not ready
    /// yet. Implementations will ensure that the current task will be notified
    /// when the next value may be ready.
    ///
    /// - `Poll::Ready(Some(val))` means that the stream has successfully
    /// produced a value, `val`, and may produce further values on subsequent
    /// `poll_next` calls.
    ///
    /// - `Poll::Ready(None)` means that the stream has terminated, and
    /// `poll_next` should not be invoked again.
    ///
    /// # Panics
    ///
    /// Once a stream is finished, i.e. `Ready(None)` has been returned, further
    /// calls to `poll_next` may result in a panic or other "bad behavior".  If
    /// this is difficult to guard against then the `fuse` adapter can be used
    /// to ensure that `poll_next` always returns `Ready(None)` in subsequent
    /// calls.
    fn poll_next(
        self: Pin<&mut Self>,
        lw: &LocalWaker,
    ) -> Poll<Option<Self::Item>>;
}

impl<'a, S: ?Sized + Stream + Unpin> Stream for &'a mut S {
    type Item = S::Item;

    fn poll_next(
        mut self: Pin<&mut Self>,
        lw: &LocalWaker,
    ) -> Poll<Option<Self::Item>> {
        S::poll_next(Pin::new(&mut **self), lw)
    }
}

impl<P> Stream for Pin<P>
where
    P: ops::DerefMut,
    P::Target: Stream,
{
    type Item = <P::Target as Stream>::Item;

    fn poll_next(
        self: Pin<&mut Self>,
        lw: &LocalWaker,
    ) -> Poll<Option<Self::Item>> {
        Pin::get_mut(self).as_mut().poll_next(lw)
    }
}

#[cfg(feature = "either")]
impl<A, B> Stream for Either<A, B>
    where A: Stream,
          B: Stream<Item = A::Item>
{
    type Item = A::Item;

    fn poll_next(self: Pin<&mut Self>, lw: &LocalWaker) -> Poll<Option<A::Item>> {
        unsafe {
            match Pin::get_mut_unchecked(self) {
                Either::Left(a) => Pin::new_unchecked(a).poll_next(lw),
                Either::Right(b) => Pin::new_unchecked(b).poll_next(lw),
            }
        }
    }
}

/// A convenience for streams that return `Result` values that includes
/// a variety of adapters tailored to such futures.
pub trait TryStream {
    /// The type of successful values yielded by this future
    type Ok;

    /// The type of failures yielded by this future
    type Error;

    /// Poll this `TryStream` as if it were a `Stream`.
    ///
    /// This method is a stopgap for a compiler limitation that prevents us from
    /// directly inheriting from the `Stream` trait; in the future it won't be
    /// needed.
    fn try_poll_next(self: Pin<&mut Self>, lw: &LocalWaker)
        -> Poll<Option<Result<Self::Ok, Self::Error>>>;
}

impl<S, T, E> TryStream for S
    where S: Stream<Item = Result<T, E>>
{
    type Ok = T;
    type Error = E;

    fn try_poll_next(self: Pin<&mut Self>, lw: &LocalWaker)
        -> Poll<Option<Result<Self::Ok, Self::Error>>>
    {
        self.poll_next(lw)
    }
}

if_std! {
    use std::boxed::Box;

    impl<S: ?Sized + Stream + Unpin> Stream for Box<S> {
        type Item = S::Item;

        fn poll_next(
            mut self: Pin<&mut Self>,
            lw: &LocalWaker,
        ) -> Poll<Option<Self::Item>> {
            Pin::new(&mut **self).poll_next(lw)
        }
    }

    impl<S: Stream> Stream for ::std::panic::AssertUnwindSafe<S> {
        type Item = S::Item;

        fn poll_next(
            self: Pin<&mut Self>,
            lw: &LocalWaker,
        ) -> Poll<Option<S::Item>> {
            unsafe { Pin::map_unchecked_mut(self, |x| &mut x.0) }.poll_next(lw)
        }
    }

    impl<T: Unpin> Stream for ::std::collections::VecDeque<T> {
        type Item = T;

        fn poll_next(
            mut self: Pin<&mut Self>,
            _lw: &LocalWaker,
        ) -> Poll<Option<Self::Item>> {
            Poll::Ready(self.pop_front())
        }
    }
}
