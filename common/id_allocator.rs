//! Data structure to produce unique IDs.

use std::default::Default;
use std::ops::Add;
use std::sync::Mutex;

/// Data structure to produce unique IDs.
pub struct T<Id> {
  next: Id,
}

impl<Id> T<Id> where
  Id : Clone + Add<u32, Output=Id>,
{
  /// Produce an Id that hasn't been produced yet by this object.
  pub fn allocate(&mut self) -> Id {
    let ret = self.next.clone();
    self.next = self.next.clone() + 1;
    ret
  }
}

/// Allocates through a mutex.
pub fn allocate<Id: Clone + Add<u32, Output=Id>>(x: &Mutex<T<Id>>) -> Id {
  x.lock().unwrap().allocate()
}

#[allow(missing_docs)]
pub fn new<Id>() -> T<Id> where
  Id : Default
{
  T {
    next: Default::default()
  }
}
