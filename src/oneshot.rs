//! Oneshot channel.
//!
//! The sender of a oneshot channel can send at most one message to the corresponding receiver.
//!
//! # Examples
//!
//! ```
//! use std::thread;
//! use nbchan::oneshot::{self, TryRecvError};
//!
//! let (tx, mut rx) = oneshot::channel();
//!
//! // Sender
//! thread::spawn(|| { tx.send(10).unwrap(); });
//!
//! // Receiver
//! loop {
//!     match rx.try_recv() {
//!         Ok(v) => {
//!             assert_eq!(v, 10);
//!             break;
//!         }
//!         Err(e) => {
//!             assert_eq!(e, TryRecvError::Empty);
//!         }
//!     }
//!  }
//! ```
#[doc(no_inline)]
pub use std::sync::mpsc::{SendError, TryRecvError};

use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

/// Creates a new asynchronous oneshot channel, returning the sender/receiver halves.
///
/// Both sending and receiving will not block the calling thread.
///
/// # Examples
///
/// ```
/// use std::thread;
/// use nbchan::oneshot::channel;
///
/// // tx is the sending half (tx for transmission), and rx is the receiving
/// // half (rx for receiving).
/// let (tx, mut rx) = channel();
///
/// # fn expensive_computation() -> usize { 0 }
/// // Spawn off an expensive computation
/// thread::spawn(move|| {
///     tx.send(expensive_computation()).unwrap();
/// });
///
/// // Do some useful work for awhile
///
/// // Let's see what that answer was
/// loop {
///     if let Ok(v) = rx.try_recv() {
///         println!("{:?}", v);
///         break;
///     }
/// }
/// ```
pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let (shared0, shared1) = SharedBox::allocate();
    let tx = Sender(shared0);
    let rx = Receiver(shared1);
    (tx, rx)
}

/// The sending-half of an asynchronous oneshot channel.
#[derive(Debug)]
pub struct Sender<T>(SharedBox<T>);
impl<T> Sender<T> {
    /// Attempts to send a value on this channel, returning it back if it could not be sent.
    ///
    /// This method will never block the current thread.
    ///
    /// # Examples
    ///
    /// ```
    /// use nbchan::oneshot::channel;
    ///
    /// // This send is always successful
    /// let (tx, _rx) = channel();
    /// tx.send(1).unwrap();
    ///
    /// // This send will fail because the receiver is gone
    /// let (tx, rx) = channel();
    /// drop(rx);
    /// assert_eq!(tx.send(1).unwrap_err().0, 1);
    /// ```
    pub fn send(mut self, t: T) -> Result<(), SendError<T>> {
        let new = into_raw_ptr(t);
        let old = self.0.swap(new);
        if old == mark_empty() {
            // Secceeded.
            self.0.abandon();
            Ok(())
        } else {
            // Failed; the receiver already has dropped.
            debug_assert_eq!(old, mark_dropped());
            let t = from_raw_ptr(self.0.load());
            self.0.release();
            Err(SendError(t))
        }
    }
}
impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        if self.0.is_available() {
            // This channel has not been used.
            let old = self.0.swap(mark_dropped());
            if old == mark_dropped() {
                // The peer (i.e., receiver) dropped first.
                self.0.release();
            }
        }
    }
}
unsafe impl<T: Send> Send for Sender<T> {}

/// The receiving-half of an asynchronous oneshot channel.
#[derive(Debug)]
pub struct Receiver<T>(SharedBox<T>);
impl<T> Receiver<T> {
    /// Attempts to return a pending value on this receiver without blocking.
    pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
        if !self.0.is_available() {
            return Err(TryRecvError::Disconnected);
        }

        let ptr = self.0.load();
        if ptr == mark_empty() {
            Err(TryRecvError::Empty)
        } else if ptr == mark_dropped() {
            self.0.release();
            Err(TryRecvError::Disconnected)
        } else {
            let t = from_raw_ptr(ptr);
            self.0.release();
            Ok(t)
        }
    }
}
impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        if self.0.is_available() {
            let old = self.0.swap(mark_dropped());
            if old != mark_empty() {
                // The peer (i.e., sender) dropped first.
                if old != mark_dropped() {
                    // The channel has an unreceived item.
                    let _t = from_raw_ptr(old);
                }
                self.0.release();
            }
        }
    }
}
unsafe impl<T: Send> Send for Receiver<T> {}

#[derive(Debug, Clone)]
struct SharedBox<T>(*mut AtomicPtr<T>);
impl<T> SharedBox<T> {
    #[inline]
    pub fn allocate() -> (Self, Self) {
        let ptr = into_raw_ptr(AtomicPtr::default());
        (SharedBox(ptr), SharedBox(ptr))
    }

    #[inline]
    pub fn release(&mut self) {
        debug_assert_ne!(self.0, ptr::null_mut());
        let _ = from_raw_ptr(self.0);
        self.0 = ptr::null_mut();
    }

    #[inline]
    pub fn abandon(&mut self) {
        debug_assert_ne!(self.0, ptr::null_mut());
        self.0 = ptr::null_mut();
    }

    #[inline]
    pub fn is_available(&self) -> bool {
        !self.0.is_null()
    }

    #[inline]
    pub fn swap(&self, value: *mut T) -> *mut T {
        debug_assert_ne!(self.0, ptr::null_mut());
        unsafe { &*self.0 }.swap(value, Ordering::SeqCst)
    }

    #[inline]
    pub fn load(&self) -> *mut T {
        unsafe { &*self.0 }.load(Ordering::SeqCst)
    }
}
unsafe impl<T: Send> Send for SharedBox<T> {}

#[inline]
fn mark_dropped<T>() -> *mut T {
    static MARK_DROPPED: &u8 = &0;
    MARK_DROPPED as *const _ as _
}

#[inline]
fn mark_empty<T>() -> *mut T {
    ptr::null_mut()
}

#[inline]
fn into_raw_ptr<T>(t: T) -> *mut T {
    Box::into_raw(Box::new(t))
}

#[inline]
fn from_raw_ptr<T>(ptr: *mut T) -> T {
    unsafe { *Box::from_raw(ptr) }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn send_and_recv_succeeds() {
        let (tx, mut rx) = channel();
        tx.send(1).unwrap();
        assert_eq!(rx.try_recv(), Ok(1));
    }

    #[test]
    fn send_succeeds() {
        let (tx, rx) = channel();
        tx.send(1).unwrap();
        drop(rx);
    }

    #[test]
    fn send_fails() {
        let (tx, rx) = channel();
        drop(rx);
        assert_eq!(tx.send(1), Err(SendError(1)));
    }

    #[test]
    fn recv_fails() {
        let (tx, mut rx) = channel::<()>();
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
        drop(tx);
        assert_eq!(rx.try_recv(), Err(TryRecvError::Disconnected));
    }

    #[test]
    fn unused() {
        channel::<()>();
    }
}
