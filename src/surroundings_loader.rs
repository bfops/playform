use lod::OwnerId;
use std::cmp::max;
use std::collections::RingBuf;
use std::num::{Float, SignedInt};
use surroundings_iter::SurroundingsIter;
use terrain::terrain_block::BlockPosition;
use time;

// Rough budget (in microseconds) for how long block updating can take PER SurroundingsLoader.
pub const BLOCK_UPDATE_BUDGET: u64 = 20000;

pub fn radius_between(p1: &BlockPosition, p2: &BlockPosition) -> i32 {
  let dx = (p1.as_pnt().x - p2.as_pnt().x).abs();
  let dy = (p1.as_pnt().y - p2.as_pnt().y).abs();
  let dz = (p1.as_pnt().z - p2.as_pnt().z).abs();
  max(max(dx, dy), dz)
}

// TODO: Should this use a trait instead of boxed closures?

pub enum LODChange {
  Load(BlockPosition, i32, OwnerId),
  Unload(BlockPosition, OwnerId),
}

/// Keep surroundings loaded around a given world position.
pub struct SurroundingsLoader<'a> {
  pub id: OwnerId,
  pub last_position: Option<BlockPosition>,

  pub max_load_distance: i32,
  pub to_load: Option<SurroundingsIter>,

  pub to_recheck: RingBuf<BlockPosition>,
  pub lod_changes: Box<FnMut(&BlockPosition, &BlockPosition) -> Vec<BlockPosition> + 'a>,
}

impl<'a> SurroundingsLoader<'a> {
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
        let block_position =
          match self.to_load.as_mut().unwrap().next() {
            None => break,
            Some(p) => p,
          };

        let distance = self.to_load.as_ref().unwrap().next_distance;
        lod_change(LODChange::Load(block_position, distance, self.id));
      }
    }
  }
}
