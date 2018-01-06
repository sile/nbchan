//! Multi-producer, single-consumer FIFO channel.
use std::cell::UnsafeCell;
use std::fmt;
use std::sync::mpsc::{SendError, TryRecvError, TrySendError};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use queue::{self, QueueHead, QueueTail};

/// Creates a new asynchronous channel, returning the sender/receiver halves.
pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let (head, tail) = queue::fifo();
    let queue_len = Arc::default();
    (
        Sender { tail },
        Receiver {
            head: UnsafeCell::new(head),
            queue_len,
        },
    )
}

/// Creates a new synchronous, bounded channel.
pub fn sync_channel<T>(bound: usize) -> (SyncSender<T>, Receiver<T>) {
    let (head, tail) = queue::fifo();
    let queue_len = Arc::default();
    (
        SyncSender {
            inner: Sender { tail },
            queue_len: Arc::clone(&queue_len),
            queue_capacity: bound,
        },
        Receiver {
            head: UnsafeCell::new(head),
            queue_len,
        },
    )
}

/// The sending-half of an asynchronous channel.
pub struct Sender<T> {
    tail: QueueTail<T>,
}
impl<T> Sender<T> {
    /// Attempts to send a value on this channel, returning it back if it could not be sent.
    ///
    /// This method will never block the current thread.
    pub fn send(&self, item: T) -> Result<(), SendError<T>> {
        if let Some(item) = self.tail.enqueue(item) {
            Err(SendError(item))
        } else {
            Ok(())
        }
    }
}
unsafe impl<T: Send> Send for Sender<T> {}
unsafe impl<T: Send> Sync for Sender<T> {}
impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Sender {
            tail: self.tail.clone(),
        }
    }
}
impl<T> fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Sender {{ .. }}")
    }
}

/// The sending-half of an asynchronous channel.
pub struct SyncSender<T> {
    inner: Sender<T>,
    queue_len: Arc<AtomicUsize>,
    queue_capacity: usize,
}
impl<T> SyncSender<T> {
    /// Attempts to send a value on this channel.
    ///
    /// This method will never block the current thread.
    pub fn try_send(&self, item: T) -> Result<(), TrySendError<T>> {
        let len = self.queue_len.fetch_add(1, Ordering::SeqCst);
        if len >= self.queue_capacity {
            self.queue_len.fetch_sub(1, Ordering::SeqCst);
            Err(TrySendError::Full(item))
        } else if let Err(SendError(item)) = self.inner.send(item) {
            self.queue_len.fetch_sub(1, Ordering::SeqCst);
            Err(TrySendError::Disconnected(item))
        } else {
            Ok(())
        }
    }
}
unsafe impl<T: Send> Send for SyncSender<T> {}
unsafe impl<T: Send> Sync for SyncSender<T> {}
impl<T> Clone for SyncSender<T> {
    fn clone(&self) -> Self {
        SyncSender {
            inner: self.inner.clone(),
            queue_len: Arc::clone(&self.queue_len),
            queue_capacity: self.queue_capacity,
        }
    }
}
impl<T> fmt::Debug for SyncSender<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SyncSender {{ .. }}")
    }
}

/// The receiving-half of an asynchronous channel.
pub struct Receiver<T> {
    head: UnsafeCell<QueueHead<T>>,
    queue_len: Arc<AtomicUsize>,
}
impl<T> Receiver<T> {
    /// Attempts to return a pending value on this receiver without blocking.
    ///
    /// This method will never block the current thread.
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        let head = unsafe { &mut *self.head.get() };
        if let Some(item) = head.dequeue() {
            self.queue_len.fetch_sub(1, Ordering::SeqCst);
            Ok(item)
        } else if head.is_tail_alive() {
            Err(TryRecvError::Empty)
        } else {
            Err(TryRecvError::Disconnected)
        }
    }
}
unsafe impl<T: Send> Send for Receiver<T> {}
impl<T> fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Receiver {{ .. }}")
    }
}

#[cfg(test)]
mod test {
    use std::mem;
    use std::sync::mpsc::{SendError, TryRecvError, TrySendError};
    use super::*;

    #[test]
    fn async_channel_works() {
        let (tx, rx) = channel::<usize>();
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));

        assert_eq!(tx.send(3), Ok(()));
        assert_eq!(rx.try_recv(), Ok(3));

        mem::drop(tx);
        assert_eq!(rx.try_recv(), Err(TryRecvError::Disconnected));

        let (tx, _) = channel::<usize>();
        assert_eq!(tx.send(3), Err(SendError(3)));
    }

    #[test]
    fn sync_channel_works() {
        let (tx, rx) = sync_channel::<usize>(1);
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));

        assert_eq!(tx.try_send(3), Ok(()));
        assert_eq!(tx.try_send(4), Err(TrySendError::Full(4)));
        assert_eq!(rx.try_recv(), Ok(3));
        assert_eq!(tx.try_send(4), Ok(()));

        mem::drop(tx);
        assert_eq!(rx.try_recv(), Ok(4));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Disconnected));

        let (tx, _) = sync_channel::<usize>(1);
        assert_eq!(tx.try_send(3), Err(TrySendError::Disconnected(3)));
    }
}
