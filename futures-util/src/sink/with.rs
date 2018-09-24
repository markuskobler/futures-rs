use core::marker::{Unpin, PhantomData};
use core::mem;
use core::pin::Pin;
use futures_core::future::Future;
use futures_core::stream::Stream;
use futures_core::task::{self, Poll};
use futures_sink::Sink;
use pin_utils::{unsafe_pinned, unsafe_unpinned};

/// Sink for the `Sink::with` combinator, chaining a computation to run *prior*
/// to pushing a value into the underlying sink.
#[derive(Debug)]
#[must_use = "sinks do nothing unless polled"]
pub struct With<Si, U, Fut, F>
    where Si: Sink,
          F: FnMut(U) -> Fut,
          Fut: Future,
{
    sink: Si,
    f: F,
    state: State<Fut, Si::SinkItem>,
    _phantom: PhantomData<fn(U)>,
}

impl<Si, U, Fut, F> With<Si, U, Fut, F>
where Si: Sink,
      F: FnMut(U) -> Fut,
      Fut: Future,
{
    unsafe_pinned!(sink: Si);
    unsafe_unpinned!(f: F);
    unsafe_pinned!(state: State<Fut, Si::SinkItem>);

    pub(super) fn new<E>(sink: Si, f: F) -> With<Si, U, Fut, F>
        where Si: Sink,
            F: FnMut(U) -> Fut,
            Fut: Future<Output = Result<Si::SinkItem, E>>,
            E: From<Si::SinkError>,
    {
        With {
            state: State::Empty,
            sink,
            f,
            _phantom: PhantomData,
        }
    }
}

impl<Si, U, Fut, F> Unpin for With<Si, U, Fut, F>
where Si: Sink + Unpin,
      F: FnMut(U) -> Fut,
      Fut: Future + Unpin,
{}

#[derive(Debug)]
enum State<Fut, T> {
    Empty,
    Process(Fut),
    Buffered(T),
}

impl<Fut, T> State<Fut, T> {
    #[allow(clippy::needless_lifetimes)] // https://github.com/rust-lang/rust/issues/52675
    #[allow(clippy::wrong_self_convention)]
    fn as_pin_mut<'a>(
        self: Pin<&'a mut Self>,
    ) -> State<Pin<&'a mut Fut>, Pin<&'a mut T>> {
        unsafe {
            match Pin::get_mut_unchecked(self) {
                State::Empty =>
                    State::Empty,
                State::Process(fut) =>
                    State::Process(Pin::new_unchecked(fut)),
                State::Buffered(item) =>
                    State::Buffered(Pin::new_unchecked(item)),
            }
        }
    }
}

// Forwarding impl of Stream from the underlying sink
impl<S, U, Fut, F> Stream for With<S, U, Fut, F>
    where S: Stream + Sink,
          F: FnMut(U) -> Fut,
          Fut: Future
{
    type Item = S::Item;

    fn poll_next(
        mut self: Pin<&mut Self>,
        lw: &LocalWaker,
    ) -> Poll<Option<S::Item>> {
        self.sink().poll_next(cx)
    }
}

impl<Si, U, Fut, F, E> With<Si, U, Fut, F>
    where Si: Sink,
          F: FnMut(U) -> Fut,
          Fut: Future<Output = Result<Si::SinkItem, E>>,
          E: From<Si::SinkError>,
{
    /// Get a shared reference to the inner sink.
    pub fn get_ref(&self) -> &Si {
        &self.sink
    }

    /// Get a mutable reference to the inner sink.
    pub fn get_mut(&mut self) -> &mut Si {
        &mut self.sink
    }

    /// Consumes this combinator, returning the underlying sink.
    ///
    /// Note that this may discard intermediate state of this combinator, so
    /// care should be taken to avoid losing resources when this is called.
    pub fn into_inner(self) -> Si {
        self.sink
    }

    fn poll(
        self: &mut Pin<&mut Self>,
        lw: &LocalWaker,
    ) -> Poll<Result<(), E>> {
        let buffered = match self.state().as_pin_mut() {
            State::Empty => return Poll::Ready(Ok(())),
            State::Process(fut) => Some(try_ready!(fut.poll(cx))),
            State::Buffered(_) => None,
        };
        if let Some(buffered) = buffered {
            Pin::set(self.state(), State::Buffered(buffered));
        }
        if let State::Buffered(item) = unsafe { mem::replace(Pin::get_mut_unchecked(self.state()), State::Empty) } {
            Poll::Ready(self.sink().start_send(item).map_err(Into::into))
        } else {
            unreachable!()
        }
    }
}

impl<Si, U, Fut, F, E> Sink for With<Si, U, Fut, F>
    where Si: Sink,
          F: FnMut(U) -> Fut,
          Fut: Future<Output = Result<Si::SinkItem, E>>,
          E: From<Si::SinkError>,
{
    type SinkItem = U;
    type SinkError = E;

    fn poll_ready(
        mut self: Pin<&mut Self>,
        lw: &LocalWaker,
    ) -> Poll<Result<(), Self::SinkError>> {
        self.poll(cx)
    }

    fn start_send(
        mut self: Pin<&mut Self>,
        item: Self::SinkItem,
    ) -> Result<(), Self::SinkError> {
        let item = (self.f())(item);
        Pin::set(self.state(), State::Process(item));
        Ok(())
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        lw: &LocalWaker,
    ) -> Poll<Result<(), Self::SinkError>> {
        try_ready!(self.poll(cx));
        try_ready!(self.sink().poll_flush(cx));
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        lw: &LocalWaker,
    ) -> Poll<Result<(), Self::SinkError>> {
        try_ready!(self.poll(cx));
        try_ready!(self.sink().poll_close(cx));
        Poll::Ready(Ok(()))
    }
}
