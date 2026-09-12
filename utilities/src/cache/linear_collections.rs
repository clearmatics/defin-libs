//! A few linear collections built on top of vector.

use std::collections::VecDeque;
use core::num::NonZeroUsize;
use std::ops::Deref;

/// A [`CappedQueue`] with fix capacity.
/// The oldest item might be evicted from the queue.

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CappedQueue<T> {
    deque: VecDeque<T>,
    capacity: NonZeroUsize,
}

impl<T> CappedQueue<T> {
    /// Create an CappedQueue with the given capacity.
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            deque: VecDeque::with_capacity(capacity.get()),
            capacity,
        }
    }

    /// Return the capacity.
    pub fn capacity(&self) -> NonZeroUsize {
        self.capacity
    }

    /// Append an item, remove and return the oldest item if it's full.
    pub fn append(&mut self, i: T) -> Option<T> {
        let to_remove = if self.deque.len() == self.capacity.get() {
            self.deque.pop_front()
        } else {
            None
        };

        self.deque.push_back(i);
        to_remove
    }

    /// Remove the oldest item.
    pub fn pop_front(&mut self) -> Option<T> {
        self.deque.pop_front()
    }

    /// Consumes the queue and returns the queue.
    pub fn into_queue(self) -> VecDeque<T> {
        self.deque
    }
}

impl<T> Deref for CappedQueue<T> {
    type Target = VecDeque<T>;

    fn deref(&self) -> &Self::Target {
        &self.deque
    }
}

impl<T> AsRef<VecDeque<T>> for CappedQueue<T> {
    fn as_ref(&self) -> &VecDeque<T> {
        &self.deque
    }
}

impl<T> From<CappedQueue<T>> for VecDeque<T> {
    fn from(deque: CappedQueue<T>) -> Self {
        deque.deque
    }
}

