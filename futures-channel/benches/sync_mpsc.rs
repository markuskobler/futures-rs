#![feature(test, futures_api, pin, arbitrary_self_types)]

use futures::ready;
use futures::channel::mpsc::{self, Sender, UnboundedSender};
use futures::executor::LocalPool;
use futures::stream::{Stream, StreamExt};
use futures::sink::Sink;
use futures::task::{self, Poll, Wake, LocalWaker};
use std::pin::Pin;
use std::sync::Arc;
use test::Bencher;

fn notify_noop() -> LocalWaker {
    struct Noop;

    impl Wake for Noop {
        fn wake(_: &Arc<Self>) {}
    }

    task::local_waker_from_nonlocal(Arc::new(Noop))
}

fn noop_lw(f: impl FnOnce(&LocalWaker)) {
    let pool = LocalPool::new();
    let mut spawn = pool.spawner();
    let waker = notify_noop();
    let lw = &LocalWaker::new(&waker, &mut spawn);
    f(lw)
}

/// Single producer, single consumer
#[bench]
fn unbounded_1_tx(b: &mut Bencher) {
    noop_lw(|lw| {
        b.iter(|| {
            let (tx, mut rx) = mpsc::unbounded();

            // 1000 iterations to avoid measuring overhead of initialization
            // Result should be divided by 1000
            for i in 0..1000 {

                // Poll, not ready, park
                assert_eq!(Poll::Pending, rx.poll_next_unpin(lw));

                UnboundedSender::unbounded_send(&tx, i).unwrap();

                // Now poll ready
                assert_eq!(Poll::Ready(Some(i)), rx.poll_next_unpin(lw));
            }
        })
    })
}

/// 100 producers, single consumer
#[bench]
fn unbounded_100_tx(b: &mut Bencher) {
    noop_lw(|lw| {
        b.iter(|| {
            let (tx, mut rx) = mpsc::unbounded();

            let tx: Vec<_> = (0..100).map(|_| tx.clone()).collect();

            // 1000 send/recv operations total, result should be divided by 1000
            for _ in 0..10 {
                for i in 0..tx.len() {
                    assert_eq!(Poll::Pending, rx.poll_next_unpin(lw));

                    UnboundedSender::unbounded_send(&tx[i], i).unwrap();

                    assert_eq!(Poll::Ready(Some(i)), rx.poll_next_unpin(lw));
                }
            }
        })
    })
}

#[bench]
fn unbounded_uncontended(b: &mut Bencher) {
    noop_lw(|lw| {
        b.iter(|| {
            let (tx, mut rx) = mpsc::unbounded();

            for i in 0..1000 {
                UnboundedSender::unbounded_send(&tx, i).expect("send");
                // No need to create a task, because poll is not going to park.
                assert_eq!(Poll::Ready(Some(i)), rx.poll_next_unpin(lw));
            }
        })
    })
}


/// A Stream that continuously sends incrementing number of the queue
struct TestSender {
    tx: Sender<u32>,
    last: u32, // Last number sent
}

// Could be a Future, it doesn't matter
impl Stream for TestSender {
    type Item = u32;

    fn poll_next(mut self: Pin<&mut Self>, lw: &LocalWaker)
        -> Poll<Option<Self::Item>>
    {
        let this = &mut *self;
        let mut tx = Pin::new(&mut this.tx);

        ready!(tx.as_mut().poll_ready(lw)).unwrap();
        tx.as_mut().start_send(this.last + 1).unwrap();
        this.last += 1;
        assert_eq!(Poll::Ready(Ok(())), tx.as_mut().poll_flush(lw));
        Poll::Ready(Some(this.last))
    }
}

/// Single producers, single consumer
#[bench]
fn bounded_1_tx(b: &mut Bencher) {
    noop_lw(|lw| {
        b.iter(|| {
            let (tx, mut rx) = mpsc::channel(0);

            let mut tx = TestSender { tx, last: 0 };

            for i in 0..1000 {
                assert_eq!(Poll::Ready(Some(i + 1)), tx.poll_next_unpin(lw));
                assert_eq!(Poll::Pending, tx.poll_next_unpin(lw));
                assert_eq!(Poll::Ready(Some(i + 1)), rx.poll_next_unpin(lw));
            }
        })
    })
}

/// 100 producers, single consumer
#[bench]
fn bounded_100_tx(b: &mut Bencher) {
    noop_lw(|lw| {
        b.iter(|| {
            // Each sender can send one item after specified capacity
            let (tx, mut rx) = mpsc::channel(0);

            let mut tx: Vec<_> = (0..100).map(|_| {
                TestSender {
                    tx: tx.clone(),
                    last: 0
                }
            }).collect();

            for i in 0..10 {
                for j in 0..tx.len() {
                    // Send an item
                    assert_eq!(Poll::Ready(Some(i + 1)), tx[j].poll_next_unpin(lw));
                    // Then block
                    assert_eq!(Poll::Pending, tx[j].poll_next_unpin(lw));
                    // Recv the item
                    assert_eq!(Poll::Ready(Some(i + 1)), rx.poll_next_unpin(lw));
                }
            }
        })
    })
}
