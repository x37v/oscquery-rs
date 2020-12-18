//! Implementations of Get and Set for all `atomic::Atomic<T: Copy + Send>` types.
use super::*;
use ::atomic::{Atomic, Ordering};

//TODO figure out ordering
const LOAD_ORDERING: Ordering = Ordering::SeqCst;
const STORE_ORDERING: Ordering = Ordering::SeqCst;

/// Implement Get<T> for Atomic<T>
impl<T> Get<T> for Atomic<T>
where
    T: Copy + Send,
{
    fn get(&self) -> T {
        self.load(LOAD_ORDERING)
    }
}

/// Implement Set<T> for Atomic<T>
impl<T> Set<T> for Atomic<T>
where
    T: Copy + Send,
{
    fn set(&self, value: T) {
        self.store(value, STORE_ORDERING);
    }
}
