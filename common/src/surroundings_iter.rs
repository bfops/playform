//! This iterator emits `BlockPositions` in cube-shaped layers outward around a point.

use cube_shell::cube_shell;
use std::collections::VecDeque;
use block_position::BlockPosition;

#[cfg(test)]
use std::iter::range_inclusive;
#[cfg(test)]
use cube_shell::cube_shell_area;
#[cfg(test)]
use surroundings_loader::radius_between;

/// This iterator emits `BlockPositions` in cube-shaped layers outward around a point.
pub struct SurroundingsIter {
  center: BlockPosition,
  next_distance: i32,
  max_distance: i32,
  to_load_buffer: VecDeque<BlockPosition>,
}

impl SurroundingsIter {
  #[allow(missing_docs)]
  pub fn new(center: BlockPosition, max_distance: i32) -> SurroundingsIter {
    let mut to_load_buffer = VecDeque::new();
    to_load_buffer.push_back(center);

    SurroundingsIter {
      center: center,
      next_distance: 0,
      max_distance: max_distance,
      to_load_buffer: to_load_buffer,
    }
  }
}

impl Iterator for SurroundingsIter {
  type Item = (BlockPosition, i32);

  #[inline]
  fn next(&mut self) -> Option<(BlockPosition, i32)> {
    let distance = self.next_distance;
    self.to_load_buffer.pop_front().map(|x| {
      if self.to_load_buffer.is_empty() {
        info!("Done loading surroundings at distance {}", self.next_distance);
        if self.next_distance < self.max_distance {
          self.next_distance += 1;
          self.to_load_buffer.extend(
            cube_shell(&self.center, self.next_distance).into_iter()
          );
        } else {
          info!("Done loading surroundings");
        }
      }
      (x, distance)
    })
  }
}

#[test]
fn shell_ordering() {
  let center = BlockPosition::new(-1, 2, 3);
  let max_distance = 4;
  let mut iter = SurroundingsIter::new(center, max_distance);

  for distance in range_inclusive(0, max_distance) {
    for _ in 0..cube_shell_area(distance) {
      let (block_posn, d) = iter.next().unwrap();
      assert_eq!(d, distance);
      assert_eq!(radius_between(&center, &block_posn), distance);
    }
  }

  assert!(iter.next().is_none());
}
