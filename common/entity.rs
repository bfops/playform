//! Uniquely identify entities

/// Phantom types to use with `id`.
mod types {
  #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, RustcEncodable, RustcDecodable)]
  pub struct Player;

  #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, RustcEncodable, RustcDecodable)]
  pub struct Mob;
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

  impl<U> std::default::Default for T<U> {
    fn default() -> Self {
      of_u32(0)
    }
  }

  impl<U> std::ops::Add<u32> for T<U> {
    type Output = T<U>;
    fn add(self, rhs: u32) -> T<U> {
      of_u32(self.value + rhs)
    }
  }

  pub type Player = T<super::types::Player>;
  pub type Mob = T<super::types::Mob>;
}
