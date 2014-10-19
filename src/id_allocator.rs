use std::default::Default;

// Produce unique `Id`s
pub struct IdAllocator<Id> {
  next: Id,
}

impl<Id> IdAllocator<Id> where
  Id : Clone + Default + Add<u32, Id>,
{
  pub fn new() -> IdAllocator<Id> {
    IdAllocator {
      next: Default::default()
    }
  }

  // Produce an Id that hasn't been produced yet by this object.
  pub fn allocate(&mut self) -> Id {
    let ret = self.next.clone();
    self.next = self.next + 1;
    ret
  }
}
