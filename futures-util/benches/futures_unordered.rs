#![feature(test, futures_api)]

use futures::channel::oneshot;
use futures::executor::block_on;
use futures::future;
use futures::stream::{StreamExt, FuturesUnordered};
use futures::task::Poll;
use std::collections::VecDeque;
use std::thread;
use test::Bencher;

#[bench]
fn oneshots(b: &mut Bencher) {
    const NUM: usize = 10_000;

    b.iter(|| {
        let mut txs = VecDeque::with_capacity(NUM);
        let mut rxs = FuturesUnordered::new();

        for _ in 0..NUM {
            let (tx, rx) = oneshot::channel();
            txs.push_back(tx);
            rxs.push(rx);
        }

        thread::spawn(move || {
            while let Some(tx) = txs.pop_front() {
                let _ = tx.send("hello");
            }
        });

        block_on(future::poll_fn(move |lw| {
            loop {
                if let Poll::Ready(None) = rxs.poll_next_unpin(lw) {
                    break
                }
            }
            Poll::Ready(())
        }))
    });
}
