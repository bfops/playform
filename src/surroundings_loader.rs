use cube_shell::cube_shell;
use id_allocator::IdAllocator;
use terrain_block::BlockPosition;
use terrain_game_loader::{TerrainGameLoader, OwnerId};
use physics::Physics;
use state::EntityId;
use std::cmp::max;
use std::collections::RingBuf;
use std::num::Float;
use std::num::SignedInt;
use time;
use stopwatch::TimerSet;
use yaglw::gl_context::GLContext;

// Rough budget (in microseconds) for how long block updating can take PER SurroundingsLoader.
pub const BLOCK_UPDATE_BUDGET: u64 = 20000;

fn radius_between(p1: &BlockPosition, p2: &BlockPosition) -> i32 {
  let dx = (p1.as_pnt().x - p2.as_pnt().x).abs();
  let dy = (p1.as_pnt().y - p2.as_pnt().y).abs();
  let dz = (p1.as_pnt().z - p2.as_pnt().z).abs();
  max(max(dx, dy), dz)
}

/// Keep surroundings loaded around a given world position.
pub struct SurroundingsLoader<'a> {
  pub id: OwnerId,
  pub last_position: Option<BlockPosition>,
  pub lod_index: |i32|:'a -> uint,

  pub max_load_distance: i32,
  pub next_load_distance: i32,
  pub load_queue: RingBuf<BlockPosition>,

  pub loaded_vec: Vec<BlockPosition>,
  pub next_unload_index: uint,

  pub solid_boundary: Vec<BlockPosition>,
}

impl<'a> SurroundingsLoader<'a> {
  pub fn new(id: OwnerId, max_load_distance: i32, lod_index: |i32|:'a -> uint) -> SurroundingsLoader {
    assert!(max_load_distance >= 0);

    SurroundingsLoader {
      id: id,
      last_position: None,
      lod_index: lod_index,

      max_load_distance: max_load_distance,
      next_load_distance: 0,
      load_queue: RingBuf::new(),

      loaded_vec: Vec::new(),
      next_unload_index: 0,

      solid_boundary: {
        let mut b = Vec::new();
        b.push_all(cube_shell(&BlockPosition::new(0, 0, 0), 0).as_slice());
        b.push_all(cube_shell(&BlockPosition::new(0, 0, 0), 1).as_slice());
        b.push_all(cube_shell(&BlockPosition::new(0, 0, 0), 2).as_slice());
        b
      },
    }
  }

  pub fn update(
    &mut self,
    timers: &TimerSet,
    gl: &mut GLContext,
    terrain_game_loader: &mut TerrainGameLoader,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    position: BlockPosition,
  ) {
    if Some(position) != self.last_position {
      self.last_position.map(
        |last_position|
          for solid_block in
            self.solid_boundary.iter().map(|&dp| last_position + dp.as_pnt().to_vec()) {
              terrain_game_loader.remove_placeholder(physics, &solid_block, self.id);
            }
        );

      // There will make some of the remove_placeholders redundant. Fix?
      for solid_block in self.solid_boundary.iter().map(|&dp| position + dp.as_pnt().to_vec()) {
        terrain_game_loader.insert_placeholder(id_allocator, physics, &solid_block, self.id);
      }

      self.load_queue.clear();
      self.load_queue.push_back(position);

      self.next_unload_index = 0;
      self.next_load_distance = 0;
      self.last_position = Some(position);
    }

    let target_time = time::precise_time_ns() + BLOCK_UPDATE_BUDGET * 1000;
    while time::precise_time_ns() < target_time {
      if self.next_unload_index < self.loaded_vec.len() {
        let block_position = self.loaded_vec[self.next_unload_index];
        if radius_between(&position, &block_position) > self.max_load_distance {
          terrain_game_loader.unload(timers, gl, physics, &block_position, self.id);
          self.loaded_vec.swap_remove(self.next_unload_index);
        } else {
          self.next_unload_index += 1;
        }
      } else {
        let block_position =
          match self.load_queue.pop_front() {
            None => break,
            Some(block_position) => block_position,
          };

        let lod_index = (self.lod_index)(self.next_load_distance);

        terrain_game_loader.load(
          timers,
          gl,
          id_allocator,
          physics,
          &block_position,
          lod_index,
          self.id,
        );

        self.loaded_vec.push(block_position);

        if self.load_queue.is_empty() {
          debug!("Done loading surroundings at distance {}", self.next_load_distance);
          self.next_load_distance += 1;
          if self.next_load_distance <= self.max_load_distance {
            self.load_queue.extend(cube_shell(&position, self.next_load_distance).into_iter());
          } else {
            debug!("Done loading surroundings");
          }
        }
      }
    }
  }
}
