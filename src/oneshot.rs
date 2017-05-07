#[doc(no_inline)]
pub use std::sync::mpsc::{SendError, TryRecvError};

use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

static MARK_DROPPED: &u8 = &0;

#[inline]
fn mark_dropped<T>() -> *mut T {
    MARK_DROPPED as *const _ as _
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let shared = Box::into_raw(Box::new(AtomicPtr::default()));
    let tx = Sender(shared);
    let rx = Receiver(shared);
    (tx, rx)
}

#[derive(Debug)]
pub struct Sender<T>(*mut AtomicPtr<T>);
impl<T> Sender<T> {
    pub fn send(mut self, t: T) -> Result<(), SendError<T>> {
        let shared = unsafe { &*self.0 };
        let new = Box::into_raw(Box::new(t));
        let old = shared.swap(new, Ordering::SeqCst);
        if old == ptr::null_mut() {
            // Secceeded.
            self.0 = ptr::null_mut();
            Ok(())
        } else {
            // The receiver already has dropped.
            let t = unsafe { *Box::from_raw(shared.load(Ordering::SeqCst)) };
            let _ = unsafe { Box::from_raw(self.0) };
            self.0 = ptr::null_mut();
            Err(SendError(t))
        }
    }
}
impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        if self.0 != ptr::null_mut() {
            // No item has been sent.
            let shared = unsafe { &*self.0 };
            let old = shared.swap(mark_dropped(), Ordering::SeqCst);
            if old == mark_dropped() {
                // The receiver already has dropped.
                let _ = unsafe { Box::from_raw(self.0) };
            }
        }
    }
}
unsafe impl<T: Send> Send for Sender<T> {}

#[derive(Debug)]
pub struct Receiver<T>(*mut AtomicPtr<T>);
impl<T> Receiver<T> {
    pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
        if self.0 == ptr::null_mut() {
            return Err(TryRecvError::Disconnected);
        }

        let shared = unsafe { &*self.0 };
        let ptr = shared.load(Ordering::SeqCst);
        if ptr == ptr::null_mut() {
            Err(TryRecvError::Empty)
        } else if ptr == mark_dropped() {
            let _ = unsafe { Box::from_raw(self.0) };
            self.0 = ptr::null_mut();
            Err(TryRecvError::Disconnected)
        } else {
            let t = unsafe { *Box::from_raw(ptr) };
            let _ = unsafe { Box::from_raw(self.0) };
            self.0 = ptr::null_mut();
            Ok(t)
        }
    }
}
impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        if self.0 != ptr::null_mut() {
            // No item has been received.
            let shared = unsafe { &*self.0 };
            let old = shared.swap(mark_dropped(), Ordering::SeqCst);
            if old != ptr::null_mut() {
                // The sender already has dropped.
                if old != mark_dropped() {
                    // Frees unreceived item.
                    let _t = unsafe { *Box::from_raw(old) };
                }
                let _ = unsafe { Box::from_raw(self.0) };
            }
        }
    }
}
unsafe impl<T: Send> Send for Receiver<T> {}

#[cfg(test)]
mod test {
    use std::mem;
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
        mem::drop(rx);
    }

    #[test]
    fn send_fails() {
        let (tx, rx) = channel();
        mem::drop(rx);
        assert_eq!(tx.send(1), Err(SendError(1)));
    }

    #[test]
    fn recv_fails() {
        let (tx, mut rx) = channel::<()>();
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
        mem::drop(tx);
        assert_eq!(rx.try_recv(), Err(TryRecvError::Disconnected));
    }
}
