// $ rustup run nightly cargo bench
#![feature(test)]
extern crate test;
extern crate nbchan;

use std::sync::mpsc as std_mpsc;
use std::thread;
use nbchan::oneshot::{self, TryRecvError};
use test::Bencher;

#[bench]
fn create_oneshot(b: &mut Bencher) {
    b.iter(|| { oneshot::channel::<()>(); });
}

#[bench]
fn create_std_mpsc(b: &mut Bencher) {
    b.iter(|| { std_mpsc::channel::<()>(); });
}

#[bench]
fn failure_send_oneshot(b: &mut Bencher) {
    b.iter(|| {
               let (tx, _) = oneshot::channel();
               let _ = tx.send(1);
           });
}

#[bench]
fn failure_send_std_mpsc(b: &mut Bencher) {
    b.iter(|| {
               let (tx, _) = std_mpsc::channel();
               let _ = tx.send(1);
           });
}

#[bench]
fn send_recv_oneshot(b: &mut Bencher) {
    b.iter(|| {
               let (tx, mut rx) = oneshot::channel();
               tx.send(1).unwrap();
               assert!(rx.try_recv().is_ok());
           });
}

#[bench]
fn send_recv_std_mpsc(b: &mut Bencher) {
    b.iter(|| {
               let (tx, rx) = std_mpsc::channel();
               tx.send(1).unwrap();
               assert!(rx.try_recv().is_ok());
           });
}

#[bench]
fn multithread_send_recv_oneshot(b: &mut Bencher) {
    let spawn_tx_tread = || {
        let (mut txs, rxs): (Vec<_>, Vec<_>) = (0..1_000_000).map(|_| oneshot::channel()).unzip();
        let _ = thread::spawn(move || while let Some(tx) = txs.pop() {
                                  let _ = tx.send(1);
                              });
        rxs
    };
    let mut rxs = spawn_tx_tread();
    b.iter(|| if let Some(mut rx) = rxs.pop() {
               while let Err(e) = rx.try_recv() {
                   assert_eq!(e, TryRecvError::Empty);
               }
           } else {
               rxs = spawn_tx_tread();
           });
}

#[bench]
fn multithread_send_recv_std_mpsc(b: &mut Bencher) {
    let spawn_tx_tread = || {
        let (mut txs, rxs): (Vec<_>, Vec<_>) = (0..1_000_000).map(|_| std_mpsc::channel()).unzip();
        let _ = thread::spawn(move || while let Some(tx) = txs.pop() {
                                  let _ = tx.send(1);
                              });
        rxs
    };
    let mut rxs = spawn_tx_tread();
    b.iter(|| if let Some(rx) = rxs.pop() {
               while let Err(e) = rx.try_recv() {
                   assert_eq!(e, TryRecvError::Empty);
               }
           } else {
               rxs = spawn_tx_tread();
           });
}
