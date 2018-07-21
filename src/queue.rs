use std::mem;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Arc;

/// Lock-free FIFO queue.
#[inline]
pub fn fifo<T>() -> (QueueHead<T>, QueueTail<T>) {
    let initial = Box::into_raw(Box::new(NodeRef::null()));
    let tail = QueueTail::new(initial);
    let head = QueueHead::new(initial, Arc::clone(&tail.tail));
    (head, tail)
}

#[derive(Debug)]
pub struct QueueTail<T> {
    tail: Arc<AtomicPtr<NodeRef<T>>>,
}
impl<T> QueueTail<T> {
    #[inline]
    pub fn enqueue(&self, item: T) -> Option<T> {
        let next = Box::into_raw(Box::new(NodeRef::null()));
        if let Some(current_tail) = self.replace_tail(next) {
            let node = Box::into_raw(Box::new(Node { item, next }));
            unsafe { &*current_tail }.store(node);
            None
        } else {
            mem::drop(unsafe { Box::from_raw(next) });
            Some(item)
        }
    }

    #[inline]
    pub fn is_disconnected(&self) -> bool {
        self.tail.load(Ordering::SeqCst).is_null()
    }

    #[inline]
    fn new(tail: *mut NodeRef<T>) -> Self {
        QueueTail {
            tail: Arc::new(AtomicPtr::new(tail)),
        }
    }

    #[inline]
    fn replace_tail(&self, new_tail: *mut NodeRef<T>) -> Option<*mut NodeRef<T>> {
        loop {
            let old = self.tail.load(Ordering::SeqCst);
            if old.is_null() {
                return None;
            }
            if self.tail.compare_and_swap(old, new_tail, Ordering::SeqCst) == old {
                return Some(old);
            }
        }
    }
}
unsafe impl<T: Send> Send for QueueTail<T> {}
unsafe impl<T: Send> Sync for QueueTail<T> {}
impl<T> Clone for QueueTail<T> {
    fn clone(&self) -> Self {
        QueueTail {
            tail: Arc::clone(&self.tail),
        }
    }
}

#[derive(Debug)]
pub struct QueueHead<T> {
    head: *mut NodeRef<T>,
    tail: Arc<AtomicPtr<NodeRef<T>>>,
}
impl<T> QueueHead<T> {
    #[inline]
    pub fn dequeue(&mut self) -> Option<T> {
        if let Some(node) = unsafe { &*self.head }.load() {
            mem::drop(unsafe { Box::from_raw(self.head) });

            self.head = node.next;
            Some(node.item)
        } else {
            None
        }
    }

    #[inline]
    pub fn is_tail_alive(&self) -> bool {
        Arc::strong_count(&self.tail) > 1
    }

    #[inline]
    fn new(head: *mut NodeRef<T>, tail: Arc<AtomicPtr<NodeRef<T>>>) -> Self {
        QueueHead { head, tail }
    }
}
unsafe impl<T: Send> Send for QueueHead<T> {}
impl<T> Drop for QueueHead<T> {
    fn drop(&mut self) {
        let tail = self.tail.swap(ptr::null_mut(), Ordering::SeqCst);
        while self.head != tail {
            let _ = self.dequeue();
        }
        mem::drop(unsafe { Box::from_raw(self.head) });
    }
}

#[derive(Debug)]
struct NodeRef<T>(AtomicPtr<Node<T>>);
impl<T> NodeRef<T> {
    #[inline]
    fn null() -> Self {
        NodeRef(AtomicPtr::default())
    }

    #[inline]
    fn load(&self) -> Option<Node<T>> {
        let ptr = self.0.load(Ordering::SeqCst);
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { *Box::from_raw(ptr) })
        }
    }

    #[inline]
    fn store(&self, node: *mut Node<T>) {
        debug_assert!(self.0.load(Ordering::SeqCst).is_null());
        self.0.store(node, Ordering::SeqCst);
    }
}

#[derive(Debug)]
struct Node<T> {
    item: T,
    next: *mut NodeRef<T>,
}
impl<T> Node<T> {}

#[cfg(test)]
mod test {
    use super::*;
    use std::mem;
    use std::thread;

    #[test]
    fn single_enqueuing_works() {
        let (mut head, tail) = fifo();
        assert_eq!(head.dequeue(), None);

        tail.enqueue(1);
        assert_eq!(head.dequeue(), Some(1));
    }

    #[test]
    fn single_producer_works() {
        let (mut head, tail) = fifo();

        for i in 0..10 {
            tail.enqueue(i);
        }
        for i in 0..10 {
            assert_eq!(head.dequeue(), Some(i));
        }
        assert_eq!(head.dequeue(), None);
    }

    #[test]
    fn multiple_producer_works() {
        let (mut head, tail) = fifo();

        for i in 0..100 {
            let tail = tail.clone();
            thread::spawn(move || {
                for j in 0..1000 {
                    tail.enqueue(i * 1000 + j);
                }
            });
        }

        let mut values = Vec::new();
        for _ in 0..100000 {
            while head.dequeue().map(|v| values.push(v)).is_none() {}
        }
        assert_eq!(head.dequeue(), None);
        values.sort();
        assert_eq!(values, (0..100000).collect::<Vec<_>>());
    }

    #[test]
    fn consumer_dropped_works() {
        let (head, tail) = fifo();

        assert_eq!(tail.enqueue(1), None);
        mem::drop(head);
        assert_eq!(tail.enqueue(1), Some(1));
    }

    #[test]
    fn producer_dropped_works() {
        let (mut head, tail) = fifo::<()>();
        assert!(head.is_tail_alive());
        assert_eq!(head.dequeue(), None);

        mem::drop(tail);
        assert!(!head.is_tail_alive());
        assert_eq!(head.dequeue(), None);
    }
}
