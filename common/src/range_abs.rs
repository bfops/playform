//! [0, -1, 1, -2, 2, .. n] iterator.

use num::traits::{Zero, One, Signed, ToPrimitive};

/// Produce numbers in increasing absolute value order to some max, like [0, 1, -1, 2, -2].
pub struct RangeAbs<T> {
  n: T,
  max: T,
}

/// Produce numbers in increasing absolute value order to some max, like [0, 1, -1, 2, -2].
pub fn range_abs<T>(inclusive_max: T) -> RangeAbs<T>
  where T: Zero + One + Signed
{
  assert!(!inclusive_max.is_negative());
  let exclusive_max = inclusive_max + One::one();
  RangeAbs {
    n: Zero::zero(),
    max: exclusive_max,
  }
}

impl<T> Iterator for RangeAbs<T>
  where T: One + Signed + ToPrimitive + Copy
{
  type Item = T;

  fn next(&mut self) -> Option<T> {
    let n = self.n;
    if n == self.max { return None }
    if !n.is_positive() {
      self.n = -n + One::one();
    } else {
      self.n = -self.n;
    }
    Some(n)
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    let sz = 2*(self.max - self.n.abs()).to_usize().unwrap();
    (sz, Some(sz))
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
