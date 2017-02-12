//! Uniquely identify entities

/// Phantom types to use with `id`.
mod types {
  #[allow(missing_docs)]
  #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, RustcEncodable, RustcDecodable)]
  pub struct Player;
}

#[allow(missing_docs)]
pub mod id {
  use std;

  #[allow(missing_docs)]
  #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, RustcEncodable, RustcDecodable)]
  pub struct T<U> {
    units: std::marker::PhantomData<U>,
    value: u32,
  }

  fn of_u32<U>(value: u32) -> T<U> {
    T {
      value: value,
      units: std::marker::PhantomData,
    }
  }

  impl<U> T<U> {
    #[allow(missing_docs)]
    pub fn to_u32(self) -> u32 {
      self.value
    }
  }

  pub type Player = T<super::types::Player>;

  pub mod allocator {
    use std;

    /// Data structure to produce unique IDs.
    pub struct T<U> {
      units : std::marker::PhantomData<U>,
      next  : u32,
    }

    pub fn new<U>() -> T<U> {
      T {
        units : std::marker::PhantomData,
        next  : 0,
      }
    }

    impl<U> T<U> {
      /// Produce an Id that hasn't been produced yet by this object.
      pub fn allocate(&mut self) -> super::T<U> {
        let ret = super::of_u32(self.next);
        self.next = self.next + 1;
        ret
      }
    }
  }
}
