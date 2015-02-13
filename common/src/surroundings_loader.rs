//! This data structure emits messages to maintain its surrounding blocks in a desired
//! loaded state (e.g. to keep player surroundings loaded, or to keep unloaded blocks
//! solid near the player).

use lod::OwnerId;
use std::cmp::max;
use std::collections::RingBuf;
use std::num::{Float, SignedInt};
use surroundings_iter::SurroundingsIter;
use block_position::BlockPosition;
use time;

// Rough budget (in microseconds) for how long block updating can take PER SurroundingsLoader.
pub const BLOCK_UPDATE_BUDGET: u64 = 20000;

/// Find the minimum cube shell radius it would take from one point to intersect the other.
pub fn radius_between(p1: &BlockPosition, p2: &BlockPosition) -> i32 {
  let dx = (p1.as_pnt().x - p2.as_pnt().x).abs();
  let dy = (p1.as_pnt().y - p2.as_pnt().y).abs();
  let dz = (p1.as_pnt().z - p2.as_pnt().z).abs();
  max(max(dx, dy), dz)
}

/// The type of message emitted by `SurroundingsLoader`. This stream of messages maintains
/// an owner's desired surroundings.
pub enum LODChange {
  /// Acquire/update an owner's handle on a given location. The distance is also passed.
  Load(BlockPosition, i32, OwnerId),
  /// Release an owner's handle on a given location.
  Unload(BlockPosition, OwnerId),
}

// TODO: Should this use a trait instead of boxed closures?

/// Iteratively load BlockPositions in cube-shaped layers around the some point.
/// That point can be updated with calls to `update`.
/// What "load" exactly means depends on the closures provided.
pub struct SurroundingsLoader<'a> {
  id: OwnerId,
  last_position: Option<BlockPosition>,

  max_load_distance: i32,
  to_load: Option<SurroundingsIter>,

  to_recheck: RingBuf<BlockPosition>,
  lod_changes: Box<FnMut(&BlockPosition, &BlockPosition) -> Vec<BlockPosition> + 'a>,
}

impl<'a> SurroundingsLoader<'a> {
  #[allow(missing_docs)]
  pub fn new(
    id: OwnerId,
    max_load_distance: i32,
    lod_changes: Box<FnMut(&BlockPosition, &BlockPosition) -> Vec<BlockPosition> + 'a>,
  ) -> SurroundingsLoader<'a> {
    assert!(max_load_distance >= 0);

    SurroundingsLoader {
      id: id,
      last_position: None,

      to_load: None,
      max_load_distance: max_load_distance,

      to_recheck: RingBuf::new(),
      lod_changes: lod_changes,
    }
  }

  /// Update the center point around which we load, and load some more blocks.
  pub fn update<LODChangeFunc>(
    &mut self,
    position: BlockPosition,
    mut lod_change: LODChangeFunc,
  ) where
    LODChangeFunc: FnMut(LODChange),
  {
    let position_changed = Some(position) != self.last_position;
    if position_changed {
      self.to_load = Some(SurroundingsIter::new(position, self.max_load_distance));
      self.last_position.map(|last_position| {
        self.to_recheck.extend(
          (self.lod_changes)(&last_position, &position).into_iter()
        );
      });

      self.last_position = Some(position);
    }

    // TODO: figure out a better termination condition for this loop.
    let target_time = time::precise_time_ns() + BLOCK_UPDATE_BUDGET * 1000;
    while time::precise_time_ns() < target_time {
      if let Some(block_position) = self.to_recheck.pop_front() {
        let distance = radius_between(&position, &block_position);
        if distance > self.max_load_distance {
          lod_change(LODChange::Unload(block_position, self.id));
        } else {
          lod_change(LODChange::Load(block_position, distance, self.id));
        }
      } else {
        let block_position;
        let distance;
        match self.to_load.as_mut().unwrap().next() {
          None => break,
          Some((p, d)) => {
            block_position = p;
            distance = d;
          },
        };

        lod_change(LODChange::Load(block_position, distance, self.id));
      }
    }
  }
}
