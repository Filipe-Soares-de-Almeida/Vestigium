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

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Items from oldest to newest.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }
}
