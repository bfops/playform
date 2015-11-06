//! Common entity datatypes.

use std::default::Default;
use std::ops::Add;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, RustcEncodable, RustcDecodable)]
/// Unique ID for a loaded entity.
pub struct T(u32);

impl Default for T {
  fn default() -> T {
    T(0)
  }
}

impl Add<u32> for T {
  type Output = T;

  fn add(self, rhs: u32) -> T {
    let T(i) = self;
    T(i + rhs)
  }
}
