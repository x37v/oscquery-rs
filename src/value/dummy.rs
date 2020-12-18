//! Implementation of Set for `()` that doesn't do anything.
use super::*;

impl<T> Set<T> for () {
    ///Doesn't do anything
    fn set(&self, _value: T) {}
}
