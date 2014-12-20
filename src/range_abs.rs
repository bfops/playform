use std::uint;

/// Generate an iterator like [0, 1, -1, 2, -2, n, -n].
pub struct RangeAbs {
  n: int,
  max: int,
}

pub fn range_abs(inclusive_max: int) -> RangeAbs {
  RangeAbs {
    n: 0,
    max: inclusive_max,
  }
}

impl Iterator<int> for RangeAbs {
  fn next(&mut self) -> Option<int> {
    if self.n == 0 {
      self.n = 1;
      Some(0)
    } else {
      let r = self.n;
      if r > 0 {
        if r == self.max + 1 {
          None
        } else {
          self.n = -self.n;
          Some(r)
        }
      } else {
        self.n = -self.n + 1;
        Some(r)
      }
    }
  }

  fn size_hint(&self) -> (uint, Option<uint>) {
    (uint::MAX, None)
  }
}

#[test]
pub fn basic_test() {
  let mut range = range_abs(2);
  assert_eq!(range.next(), Some(0));
  assert_eq!(range.next(), Some(1));
  assert_eq!(range.next(), Some(-1));
  assert_eq!(range.next(), Some(2));
  assert_eq!(range.next(), Some(-2));
  assert_eq!(range.next(), None);
}
