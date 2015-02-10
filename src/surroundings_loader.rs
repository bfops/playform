use id_allocator::IdAllocator;
use lod_map::{LOD, OwnerId};
use opencl_context::CL;
use physics::Physics;
use std::cmp::max;
use std::collections::RingBuf;
use std::num::{Float, SignedInt};
use std::sync::mpsc::Sender;
use stopwatch::TimerSet;
use surroundings_iter::SurroundingsIter;
use terrain::terrain_block::BlockPosition;
use terrain::terrain_game_loader::TerrainGameLoader;
use time;
use view_update::ViewUpdate;
use world::EntityId;

// Rough budget (in microseconds) for how long block updating can take PER SurroundingsLoader.
pub const BLOCK_UPDATE_BUDGET: u64 = 20000;

pub fn radius_between(p1: &BlockPosition, p2: &BlockPosition) -> i32 {
  let dx = (p1.as_pnt().x - p2.as_pnt().x).abs();
  let dy = (p1.as_pnt().y - p2.as_pnt().y).abs();
  let dz = (p1.as_pnt().z - p2.as_pnt().z).abs();
  max(max(dx, dy), dz)
}

// TODO: This should probably use a trait instead of boxed closures.

/// Keep surroundings loaded around a given world position.
pub struct SurroundingsLoader<'a> {
  pub id: OwnerId,
  pub last_position: Option<BlockPosition>,
  pub lod: Box<FnMut(i32) -> LOD + 'a>,

  pub max_load_distance: i32,
  pub to_load: Option<SurroundingsIter>,

  pub to_recheck: RingBuf<BlockPosition>,
  pub lod_changes: Box<FnMut(&BlockPosition, &BlockPosition) -> Vec<BlockPosition> + 'a>,
}

impl<'a> SurroundingsLoader<'a> {
  pub fn new(
    id: OwnerId,
    max_load_distance: i32,
    lod: Box<FnMut(i32) -> LOD + 'a>,
    lod_changes: Box<FnMut(&BlockPosition, &BlockPosition) -> Vec<BlockPosition> + 'a>,
  ) -> SurroundingsLoader<'a> {
    assert!(max_load_distance >= 0);

    SurroundingsLoader {
      id: id,
      last_position: None,
      lod: lod,

      to_load: None,
      max_load_distance: max_load_distance,

      to_recheck: RingBuf::new(),
      lod_changes: lod_changes,
    }
  }

  pub fn update(
    &mut self,
    timers: &TimerSet,
    view: &Sender<ViewUpdate>,
    cl: &CL,
    terrain_game_loader: &mut TerrainGameLoader,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    position: BlockPosition,
  ) {
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

    let target_time = time::precise_time_ns() + BLOCK_UPDATE_BUDGET * 1000;
    while time::precise_time_ns() < target_time {
      if let Some(block_position) = self.to_recheck.pop_front() {
        let distance = radius_between(&position, &block_position);
        if distance > self.max_load_distance {
          terrain_game_loader.decrease_lod(
            timers,
            view,
            cl,
            id_allocator,
            physics,
            &block_position,
            None,
            self.id,
          );
        } else {
          let lod = (self.lod)(distance);
          terrain_game_loader.decrease_lod(
            timers,
            view,
            cl,
            id_allocator,
            physics,
            &block_position,
            Some(lod),
            self.id,
          );
        }
      } else {
        let block_position =
          match self.to_load.as_mut().unwrap().next() {
            None => break,
            Some(p) => p,
          };

        let lod = (self.lod)(self.to_load.as_ref().unwrap().next_distance);

        terrain_game_loader.increase_lod(
          timers,
          view,
          cl,
          id_allocator,
          physics,
          &block_position,
          lod,
          self.id,
        );
      }
    }
  }
}
