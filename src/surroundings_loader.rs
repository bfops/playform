use id_allocator::IdAllocator;
use lod_map::{LOD, OwnerId};
use opencl_context::CL;
use physics::Physics;
use std::cmp::max;
use std::num::{Float, SignedInt};
use std::sync::mpsc::Sender;
use stopwatch::TimerSet;
use surroundings_iter::SurroundingsIter;
use terrain::terrain_block::BlockPosition;
use terrain::terrain_game_loader::TerrainGameLoader;
use time;
use view::ViewUpdate;
use world::EntityId;

// Rough budget (in microseconds) for how long block updating can take PER SurroundingsLoader.
pub const BLOCK_UPDATE_BUDGET: u64 = 20000;

pub fn radius_between(p1: &BlockPosition, p2: &BlockPosition) -> i32 {
  let dx = (p1.as_pnt().x - p2.as_pnt().x).abs();
  let dy = (p1.as_pnt().y - p2.as_pnt().y).abs();
  let dz = (p1.as_pnt().z - p2.as_pnt().z).abs();
  max(max(dx, dy), dz)
}

/// Keep surroundings loaded around a given world position.
pub struct SurroundingsLoader<'a> {
  pub id: OwnerId,
  pub last_position: Option<BlockPosition>,
  pub lod: Box<FnMut(i32) -> LOD + 'a>,

  pub max_load_distance: i32,
  pub to_load: Option<SurroundingsIter>,

  pub loaded_vec: Vec<BlockPosition>,
  // We iterate through loaded_vec, checking for things to unload.
  // This is the next position to check.
  pub next_unload_index: usize,
}

impl<'a> SurroundingsLoader<'a> {
  pub fn new(
    id: OwnerId,
    max_load_distance: i32,
    lod: Box<FnMut(i32) -> LOD + 'a>,
  ) -> SurroundingsLoader<'a> {
    assert!(max_load_distance >= 0);

    SurroundingsLoader {
      id: id,
      last_position: None,
      lod: lod,

      to_load: None,
      max_load_distance: max_load_distance,

      loaded_vec: Vec::new(),
      next_unload_index: 0,
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
      self.next_unload_index = 0;
      self.last_position = Some(position);
    }

    let target_time = time::precise_time_ns() + BLOCK_UPDATE_BUDGET * 1000;
    while time::precise_time_ns() < target_time {
      if self.next_unload_index < self.loaded_vec.len() {
        let block_position = self.loaded_vec[self.next_unload_index];
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
          self.loaded_vec.swap_remove(self.next_unload_index);
        } else {
          let lod = (self.lod)(distance);
          // This can fail; we leave it in the vec for next time.
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

          self.next_unload_index += 1;
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

        self.loaded_vec.push(block_position);
      }
    }
  }
}
