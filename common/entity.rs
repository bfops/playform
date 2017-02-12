//! Uniquely identify entities

/// Phantom types to use with `id`.
pub mod types {
  #[allow(missing_docs)]
  #[derive(Debug, Clone, Copy, RustcEncodable, RustcDecodable)]
  pub struct Player;
}

#[allow(missing_docs)]
pub mod id {
  use std;
  use std::ops::Add;

  #[allow(missing_docs)]
  #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, RustcEncodable, RustcDecodable)]
  pub struct T<U> {
    units: std::marker::PhantomData<U>,
    value: u32,
  }

  impl<U> T<U> {
    #[allow(missing_docs)]
    pub fn to_u32(self) -> u32 {
      self.value
    }
  }

  impl<U> Add<T<U>> for T<U> {
    type Output = T<U>;

    fn add(self, rhs: T<U>) -> T<U> {
      let T { units: _, value } = self;
      let T { units: _, value: rhs } = rhs;
      T {
        value: value + rhs,
        units: std::marker::PhantomData,
      }
    }
  }

  pub type Player = T<super::types::Player>;
}
