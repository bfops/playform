use cube_shell::cube_shell;
use id_allocator::IdAllocator;
use terrain_block::BlockPosition;
use terrain_game_loader::TerrainGameLoader;
use physics::Physics;
use state::EntityId;
use std::cmp::max;
use std::collections::RingBuf;
use std::num::Float;
use std::num::SignedInt;
use stopwatch::TimerSet;
use yaglw::gl_context::GLContext;

// values are approximately in microseconds, but they don't have to be.
pub const BLOCK_UPDATE_BUDGET: int = 20000;
pub const BLOCK_LOAD_COST: int = 600;
pub const BLOCK_UNLOAD_COST: int = 300;

fn radius_between(p1: &BlockPosition, p2: &BlockPosition) -> i32 {
  let dx = (p1.as_pnt().x - p2.as_pnt().x).abs();
  let dy = (p1.as_pnt().y - p2.as_pnt().y).abs();
  let dz = (p1.as_pnt().z - p2.as_pnt().z).abs();
  max(max(dx, dy), dz)
}

/// Keep surroundings loaded around a given world position.
pub struct SurroundingsLoader {
  pub last_position: Option<BlockPosition>,

  pub max_load_distance: i32,
  pub next_load_distance: i32,
  pub load_queue: RingBuf<BlockPosition>,

  pub loaded_vec: Vec<BlockPosition>,
  pub next_unload_index: uint,

  pub solid_boundary: Vec<BlockPosition>,
}

impl SurroundingsLoader {
  pub fn new(max_load_distance: i32) -> SurroundingsLoader {
    assert!(max_load_distance >= 0);

    SurroundingsLoader {
      last_position: None,

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
        b.push_all(cube_shell(&BlockPosition::new(0, 0, 0), 3).as_slice());
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
              terrain_game_loader.unload(timers, gl, physics, &solid_block);
            }
        );

      // There will be some redundant unloads above. Fix?
      for solid_block in self.solid_boundary.iter().map(|&dp| position + dp.as_pnt().to_vec()) {
        terrain_game_loader.mark_wanted(id_allocator, physics, &solid_block);
      }

      self.load_queue.clear();
      self.load_queue.push_back(position);

      self.next_unload_index = 0;
      self.next_load_distance = 0;
      self.last_position = Some(position);
    }

    let mut budget = BLOCK_UPDATE_BUDGET;
    while budget > 0 {
      if self.next_unload_index < self.loaded_vec.len() {
        let block_position = self.loaded_vec[self.next_unload_index];
        if radius_between(&position, &block_position) > self.max_load_distance {
          if terrain_game_loader.unload(timers, gl, physics, &block_position) {
            budget -= BLOCK_UNLOAD_COST;
          }

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

        if terrain_game_loader.load(
          timers,
          gl,
          id_allocator,
          physics,
          &block_position,
        ) {
          budget -= BLOCK_LOAD_COST;
        }

        self.loaded_vec.push(block_position);

        if self.load_queue.is_empty() {
          self.next_load_distance += 1;
          if self.next_load_distance <= self.max_load_distance {
            self.load_queue.extend(cube_shell(&position, self.next_load_distance).into_iter());
          }
        }
      }
    }
  }
}
