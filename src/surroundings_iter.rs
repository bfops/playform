use cube_shell::cube_shell;
use terrain_block::BlockPosition;
use std::collections::RingBuf;

pub struct SurroundingsIter {
  pub center: BlockPosition,
  pub next_distance: i32,
  pub max_distance: i32,
  pub to_load_buffer: RingBuf<BlockPosition>,
}

impl SurroundingsIter {
  pub fn new(center: BlockPosition, max_distance: i32) -> SurroundingsIter {
    let mut to_load_buffer = RingBuf::new();
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
  type Item = BlockPosition;

  #[inline]
  fn next(&mut self) -> Option<BlockPosition> {
    let r = self.to_load_buffer.pop_front();
    if r.is_some() && self.to_load_buffer.is_empty() {
      debug!("Done loading surroundings at distance {}", self.next_distance);
      if self.next_distance < self.max_distance {
        self.next_distance += 1;
        self.to_load_buffer.extend(
          cube_shell(&self.center, self.next_distance).into_iter()
        );
      } else {
        debug!("Done loading surroundings");
      }
    }
    r
  }
}
