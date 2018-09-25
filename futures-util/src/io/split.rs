use crate::lock::BiLock;
use futures_core::task::{self, Poll};
use futures_io::{AsyncRead, AsyncWrite, IoVec};
use std::io;
use std::pin::Pin;

/// The readable half of an object returned from `AsyncRead::split`.
#[derive(Debug)]
pub struct ReadHalf<T> {
    handle: BiLock<T>,
}

/// The writable half of an object returned from `AsyncRead::split`.
#[derive(Debug)]
pub struct WriteHalf<T> {
    handle: BiLock<T>,
}

fn lock_and_then<T, U, E, F>(
    lock: &BiLock<T>,
    lw: &LocalWaker,
    f: F
) -> Poll<Result<U, E>>
    where F: FnOnce(&mut T, &LocalWaker) -> Poll<Result<U, E>>
{
    match lock.poll_lock(lw) {
        // Safety: the value behind the bilock used by `ReadHalf` and `WriteHalf` is never exposed
        // as a `Pin<&mut T>` anywhere other than here as a way to get to `&mut T`.
        Poll::Ready(mut l) => f(unsafe { Pin::get_mut_unchecked(l.as_pin_mut()) }, lw),
        Poll::Pending => Poll::Pending,
    }
}

pub fn split<T: AsyncRead + AsyncWrite>(t: T) -> (ReadHalf<T>, WriteHalf<T>) {
    let (a, b) = BiLock::new(t);
    (ReadHalf { handle: a }, WriteHalf { handle: b })
}

impl<R: AsyncRead> AsyncRead for ReadHalf<R> {
    fn poll_read(&mut self, lw: &LocalWaker, buf: &mut [u8])
        -> Poll<io::Result<usize>>
    {
        lock_and_then(&self.handle, lw, |l, lw| l.poll_read(lw, buf))
    }

    fn poll_vectored_read(&mut self, lw: &LocalWaker, vec: &mut [&mut IoVec])
        -> Poll<io::Result<usize>>
    {
        lock_and_then(&self.handle, lw, |l, lw| l.poll_vectored_read(lw, vec))
    }
}

impl<W: AsyncWrite> AsyncWrite for WriteHalf<W> {
    fn poll_write(&mut self, lw: &LocalWaker, buf: &[u8])
        -> Poll<io::Result<usize>>
    {
        lock_and_then(&self.handle, lw, |l, lw| l.poll_write(lw, buf))
    }

    fn poll_vectored_write(&mut self, lw: &LocalWaker, vec: &[&IoVec])
        -> Poll<io::Result<usize>>
    {
        lock_and_then(&self.handle, lw, |l, lw| l.poll_vectored_write(lw, vec))
    }

    fn poll_flush(&mut self, lw: &LocalWaker) -> Poll<io::Result<()>> {
        lock_and_then(&self.handle, lw, |l, lw| l.poll_flush(lw))
    }

    fn poll_close(&mut self, lw: &LocalWaker) -> Poll<io::Result<()>> {
        lock_and_then(&self.handle, lw, |l, lw| l.poll_close(lw))
    }
}
