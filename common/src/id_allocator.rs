//! Data structure to produce unique IDs.

use std::default::Default;
use std::ops::Add;

/// Data structure to produce unique IDs.
pub struct IdAllocator<Id> {
  next: Id,
}

impl<Id> IdAllocator<Id> where
  Id : Clone + Default + Add<u32, Output=Id>,
{
  #[allow(missing_docs)]
  pub fn new() -> IdAllocator<Id> {
    IdAllocator {
      next: Default::default()
    }
  }

  /// Produce an Id that hasn't been produced yet by this object.
  pub fn allocate(&mut self) -> Id {
    let ret = self.next.clone();
    self.next = self.next.clone() + 1;
    ret
  }
}
