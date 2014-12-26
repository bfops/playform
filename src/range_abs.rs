use std::num::Int;
use std::uint;

/// Generate an iterator like [0, 1, -1, 2, -2, n, -n].
pub struct RangeAbs<T> {
  n: T,
  max: T,
}

pub fn range_abs<T>(inclusive_max: T) -> RangeAbs<T> 
  where T: Int
{
  assert!(inclusive_max >= Int::zero());
  RangeAbs {
    n: Int::zero(),
    max: inclusive_max,
  }
}

impl<T> Iterator<T> for RangeAbs<T> 
  where T: Int + Neg<T>
{
  fn next(&mut self) -> Option<T> {
    if self.n == Int::zero() {
      self.n = Int::one();
      Some(Int::zero())
    } else {
      let r = self.n;
      if r > Int::zero() {
        if r == self.max + Int::one() {
          None
        } else {
          self.n = -self.n;
          Some(r)
        }
      } else {
        self.n = -self.n + Int::one();
        Some(r)
      }
    }
  }

  fn size_hint(&self) -> (uint, Option<uint>) {
    (uint::MAX, None)
  }
}

#[test]
fn basic_test() {
  let mut range = range_abs(2 as i32);
  assert_eq!(range.next(), Some(0));
  assert_eq!(range.next(), Some(1));
  assert_eq!(range.next(), Some(-1));
  assert_eq!(range.next(), Some(2));
  assert_eq!(range.next(), Some(-2));
  assert_eq!(range.next(), None);
}

#[test]
fn test_range_abs_0() {
  let mut range = range_abs(0 as i32);
  assert_eq!(range.next(), Some(0));
  assert_eq!(range.next(), None);
}
