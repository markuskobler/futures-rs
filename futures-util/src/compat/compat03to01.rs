use super::Compat;
use futures::{
    task as task01, Async as Async01, AsyncSink as AsyncSink01,
    Future as Future01, Poll as Poll01, Sink as Sink01,
    StartSend as StartSend01, Stream as Stream01,
};
use futures_core::{
    task as task03, TryFuture as TryFuture03, TryStream as TryStream03,
};
use futures_sink::Sink as Sink03;
use std::{marker::Unpin, pin::Pin, sync::Arc};

impl<Fut, Sp> Future01 for Compat<Fut, Sp>
where
    Fut: TryFuture03 + Unpin,
    Sp: task03::Spawn,
{
    type Item = Fut::Ok;
    type Error = Fut::Error;

    fn poll(&mut self) -> Poll01<Self::Item, Self::Error> {
        with_context(self, |inner, cx| match inner.try_poll(cx) {
            task03::Poll::Ready(Ok(t)) => Ok(Async01::Ready(t)),
            task03::Poll::Pending => Ok(Async01::NotReady),
            task03::Poll::Ready(Err(e)) => Err(e),
        })
    }
}

impl<St, Sp> Stream01 for Compat<St, Sp>
where
    St: TryStream03 + Unpin,
    Sp: task03::Spawn,
{
    type Item = St::Ok;
    type Error = St::Error;

    fn poll(&mut self) -> Poll01<Option<Self::Item>, Self::Error> {
        with_context(self, |inner, cx| match inner.try_poll_next(cx) {
            task03::Poll::Ready(None) => Ok(Async01::Ready(None)),
            task03::Poll::Ready(Some(Ok(t))) => Ok(Async01::Ready(Some(t))),
            task03::Poll::Pending => Ok(Async01::NotReady),
            task03::Poll::Ready(Some(Err(e))) => Err(e),
        })
    }
}

impl<T, E> Sink01 for Compat<T, E>
where
    T: Sink03 + Unpin,
    E: task03::Spawn,
{
    type SinkItem = T::SinkItem;
    type SinkError = T::SinkError;

    fn start_send(
        &mut self,
        item: Self::SinkItem,
    ) -> StartSend01<Self::SinkItem, Self::SinkError> {
        with_context(self, |mut inner, cx| {
            match inner.as_mut().poll_ready(cx) {
                task03::Poll::Ready(Ok(())) => {
                    inner.start_send(item).map(|()| AsyncSink01::Ready)
                }
                task03::Poll::Pending => Ok(AsyncSink01::NotReady(item)),
                task03::Poll::Ready(Err(e)) => Err(e),
            }
        })
    }

    fn poll_complete(&mut self) -> Poll01<(), Self::SinkError> {
        with_context(self, |inner, cx| match inner.poll_flush(cx) {
            task03::Poll::Ready(Ok(())) => Ok(Async01::Ready(())),
            task03::Poll::Pending => Ok(Async01::NotReady),
            task03::Poll::Ready(Err(e)) => Err(e),
        })
    }

    fn close(&mut self) -> Poll01<(), Self::SinkError> {
        with_context(self, |inner, cx| match inner.poll_close(cx) {
            task03::Poll::Ready(Ok(())) => Ok(Async01::Ready(())),
            task03::Poll::Pending => Ok(Async01::NotReady),
            task03::Poll::Ready(Err(e)) => Err(e),
        })
    }
}

fn current_as_waker() -> task03::LocalWaker {
    let arc_waker = Arc::new(Current(task01::current()));
    task03::local_waker_from_nonlocal(arc_waker)
}

struct Current(task01::Task);

impl task03::Wake for Current {
    fn wake(arc_self: &Arc<Self>) {
        arc_self.0.notify();
    }
}

fn with_context<T, E, R, F>(compat: &mut Compat<T, E>, f: F) -> R
where
    T: Unpin,
    E: task03::Spawn,
    F: FnOnce(Pin<&mut T>, &mut task03::Context) -> R,
{
    let waker = current_as_waker();
    let spawn = compat.spawn.as_mut().unwrap();
    let mut cx = task03::Context::new(&waker, spawn);
    f(Pin::new(&mut compat.inner), &mut cx)
}
