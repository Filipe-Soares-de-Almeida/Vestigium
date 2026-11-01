use std::collections::VecDeque;

/// Fixed-size history that drops the oldest item when full.
#[derive(Debug)]
pub struct Ring<T> {
    items: VecDeque<T>,
    capacity: usize,
}

impl<T> Ring<T> {
    pub fn with_capacity(capacity: usize) -> Ring<T> {
        Ring {
            items: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, item: T) {
        if self.capacity == 0 {
            return;
        }
        if self.items.len() == self.capacity {
            self.items.pop_front();
        }
        self.items.push_back(item);
    }

    /// Drops items from the oldest end while `old` holds.
    pub fn drop_while(&mut self, old: impl Fn(&T) -> bool) {
        while self.items.front().is_some_and(&old) {
            self.items.pop_front();
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Items from oldest to newest.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> {
        self.items.iter()
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }
}
