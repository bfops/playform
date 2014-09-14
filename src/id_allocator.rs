#[deriving(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Show)]
pub struct Id(u32);

impl Id {
  pub fn none() -> Id {
    Id(0)
  }
}

impl Add<u32, Id> for Id {
  fn add(&self, rhs: &u32) -> Id {
    let Id(i) = *self;
    Id(i + *rhs)
  }
}

impl Mul<u32, Id> for Id {
  fn mul(&self, rhs: &u32) -> Id {
    let Id(i) = *self;
    Id(i * *rhs)
  }
}

// Produce unique `Id`s.
pub struct IdAllocator {
  next: Id,
}

impl IdAllocator {
  pub fn new() -> IdAllocator {
    IdAllocator { next: Id(1) }
  }

  // Produce an Id that hasn't been produced yet by this object.
  pub fn allocate(&mut self) -> Id {
    let ret = self.next;
    self.next = self.next + 1;
    ret
  }
}
