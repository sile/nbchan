// $ rustup run nightly cargo bench
#![feature(test)]
extern crate nbchan;
extern crate test;

use std::sync::mpsc as std_mpsc;
use std::thread;
use std::time::Duration;
use nbchan::mpsc;
use nbchan::oneshot::{self, TryRecvError};
use test::Bencher;

#[bench]
fn create_nbchan_oneshot(b: &mut Bencher) {
    b.iter(|| {
        oneshot::channel::<()>();
    });
}

#[bench]
fn create_nbchan_mpsc(b: &mut Bencher) {
    b.iter(|| {
        mpsc::channel::<()>();
    });
}

#[bench]
fn create_std_mpsc(b: &mut Bencher) {
    b.iter(|| {
        std_mpsc::channel::<()>();
    });
}

#[bench]
fn clone_sender_nbchan_mpsc(b: &mut Bencher) {
    let (tx, _rx) = mpsc::channel::<()>();
    b.iter(|| {
        tx.clone();
    });
}

#[bench]
fn clone_sender_std_mpsc(b: &mut Bencher) {
    let (tx, _rx) = std_mpsc::channel::<()>();
    b.iter(|| {
        tx.clone();
    });
}

#[bench]
fn oneshot_failure_send_nbchan_oneshot(b: &mut Bencher) {
    b.iter(|| {
        let (tx, _) = oneshot::channel();
        let _ = tx.send(1);
    });
}

#[bench]
fn oneshot_failure_send_nbchan_mpsc(b: &mut Bencher) {
    b.iter(|| {
        let (tx, _) = mpsc::channel();
        let _ = tx.send(1);
    });
}

#[bench]
fn oneshot_failure_send_std_mpsc(b: &mut Bencher) {
    b.iter(|| {
        let (tx, _) = std_mpsc::channel();
        let _ = tx.send(1);
    });
}

#[bench]
fn oneshot_send_recv_nbchan_oneshot(b: &mut Bencher) {
    b.iter(|| {
        let (tx, mut rx) = oneshot::channel();
        tx.send(1).unwrap();
        assert!(rx.try_recv().is_ok());
    });
}

#[bench]
fn oneshot_send_recv_nbchan_mpsc(b: &mut Bencher) {
    b.iter(|| {
        let (tx, rx) = mpsc::channel();
        tx.send(1).unwrap();
        assert!(rx.try_recv().is_ok());
    });
}

#[bench]
fn oneshot_send_recv_std_mpsc(b: &mut Bencher) {
    b.iter(|| {
        let (tx, rx) = std_mpsc::channel();
        tx.send(1).unwrap();
        assert!(rx.try_recv().is_ok());
    });
}

#[bench]
fn oneshot_multithread_send_recv_nbchan_oneshot(b: &mut Bencher) {
    let (txs_tx, txs_rx) = std_mpsc::sync_channel(2);
    let (rxs_tx, rxs_rx) = std_mpsc::sync_channel(2);
    let _ = thread::spawn(move || loop {
        let (txs, rxs): (Vec<_>, Vec<_>) = (0..100_000).map(|_| oneshot::channel()).unzip();
        if txs_tx.send(txs).is_err() {
            break;
        }
        if rxs_tx.send(rxs).is_err() {
            break;
        }
    });
    let _ = thread::spawn(move || {
        while let Ok(mut txs) = txs_rx.recv() {
            while let Some(tx) = txs.pop() {
                if tx.send(1).is_err() {
                    return;
                }
            }
        }
    });
    thread::sleep(Duration::from_millis(10));

    let mut rxs = Vec::new();
    b.iter(|| {
        if let Some(rx) = rxs.pop() {
            let mut rx: oneshot::Receiver<usize> = rx;
            while let Err(e) = rx.try_recv() {
                assert_eq!(e, TryRecvError::Empty);
            }
        } else {
            rxs = rxs_rx.recv().unwrap();
        }
    });
}

#[bench]
fn oneshot_multithread_send_recv_nbchan_mpsc(b: &mut Bencher) {
    let (txs_tx, txs_rx) = std_mpsc::sync_channel(2);
    let (rxs_tx, rxs_rx) = std_mpsc::sync_channel(2);
    let _ = thread::spawn(move || loop {
        let (txs, rxs): (Vec<_>, Vec<_>) = (0..100_000).map(|_| mpsc::channel()).unzip();
        if txs_tx.send(txs).is_err() {
            break;
        }
        if rxs_tx.send(rxs).is_err() {
            break;
        }
    });
    let _ = thread::spawn(move || {
        while let Ok(mut txs) = txs_rx.recv() {
            while let Some(tx) = txs.pop() {
                if tx.send(1).is_err() {
                    return;
                }
            }
        }
    });
    thread::sleep(Duration::from_millis(10));

    let mut rxs = Vec::new();
    b.iter(|| {
        if let Some(rx) = rxs.pop() {
            let rx: mpsc::Receiver<usize> = rx;
            while let Err(e) = rx.try_recv() {
                assert_eq!(e, TryRecvError::Empty);
            }
        } else {
            rxs = rxs_rx.recv().unwrap();
        }
    });
}

#[bench]
fn oneshot_multithread_send_recv_std_mpsc(b: &mut Bencher) {
    let (txs_tx, txs_rx) = std_mpsc::sync_channel(2);
    let (rxs_tx, rxs_rx) = std_mpsc::sync_channel(2);
    let _ = thread::spawn(move || loop {
        let (txs, rxs): (Vec<_>, Vec<_>) = (0..100_000).map(|_| std_mpsc::channel()).unzip();
        if txs_tx.send(txs).is_err() {
            break;
        }
        if rxs_tx.send(rxs).is_err() {
            break;
        }
    });
    let _ = thread::spawn(move || {
        while let Ok(mut txs) = txs_rx.recv() {
            while let Some(tx) = txs.pop() {
                if tx.send(1).is_err() {
                    return;
                }
            }
        }
    });
    thread::sleep(Duration::from_millis(10));

    let mut rxs = Vec::new();
    b.iter(|| {
        if let Some(rx) = rxs.pop() {
            let rx: std_mpsc::Receiver<usize> = rx;
            while let Err(e) = rx.try_recv() {
                assert_eq!(e, TryRecvError::Empty);
            }
        } else {
            rxs = rxs_rx.recv().unwrap();
        }
    });
}

#[bench]
fn stream_send_recv_nbchan_mpsc(b: &mut Bencher) {
    let (tx, rx) = mpsc::channel();
    for _ in 0..100 {
        let tx = tx.clone();
        thread::spawn(move || {
            for i in 0..100_000 {
                assert!(tx.send(i).is_ok());
            }
        });
    }

    let mut count = 0;
    b.iter(|| {
        count += (0..100).filter(|_| rx.try_recv().is_ok()).count();
    });
    assert_eq!(count, 100 * 100_000);
}

#[bench]
fn stream_send_recv_std_mpsc(b: &mut Bencher) {
    let (tx, rx) = std_mpsc::channel();
    for _ in 0..100 {
        let tx = tx.clone();
        thread::spawn(move || {
            for i in 0..100_000 {
                assert!(tx.send(i).is_ok());
            }
        });
    }

    let mut count = 0;
    b.iter(|| {
        count += (0..100).filter(|_| rx.try_recv().is_ok()).count();
    });
    assert_eq!(count, 100 * 100_000);
}
