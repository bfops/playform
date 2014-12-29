use cube_shell::cube_shell;
use id_allocator::IdAllocator;
use terrain_block::BlockPosition;
use terrain_game_loader::TerrainGameLoader;
use physics::Physics;
use state::EntityId;
use std::collections::HashSet;
use std::collections::RingBuf;
use std::iter::range_inclusive;
use stopwatch::TimerSet;
use yaglw::gl_context::GLContext;

#[cfg(test)]
use cube_shell::cube_shell_area;
#[cfg(test)]
use nalgebra::Pnt3;
#[cfg(test)]
use ncollide::bounding_volume::AABB;
#[cfg(test)]
use std::cmp::max;
#[cfg(test)]
use std::num::SignedInt;

// values are approximately in microseconds, but they don't have to be.
pub const BLOCK_UPDATE_BUDGET: int = 8000;
pub const BLOCK_LOAD_COST: int = 600;
pub const BLOCK_UNLOAD_COST: int = 300;

/// Keep surroundings loaded around a given world position.
pub struct SurroundingsLoader {
  pub load_queue: RingBuf<BlockPosition>,
  pub unload_queue: RingBuf<BlockPosition>,

  pub load_distance: i32,

  pub last_position: Option<BlockPosition>,
}

impl SurroundingsLoader {
  pub fn new(load_distance: i32) -> SurroundingsLoader {
    assert!(load_distance >= 0);

    SurroundingsLoader {
      load_queue: RingBuf::new(),
      unload_queue: RingBuf::new(),

      load_distance: load_distance,

      last_position: None,
    }
  }

  pub fn update<'a>(
    &mut self,
    timers: &TimerSet,
    gl: &mut GLContext,
    terrain_game_loader: &mut TerrainGameLoader<'a>,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    position: BlockPosition,
  ) {
    timers.time("update.update_queues", || {
      self.update_queues(timers, terrain_game_loader, id_allocator, physics, position);
    });
    timers.time("update.load_some", || {
      self.load_some(timers, gl, terrain_game_loader, id_allocator, physics);
    });
  }

  #[inline]
  fn update_queues<'a>(
    &mut self,
    timers: &TimerSet,
    terrain_game_loader: &mut TerrainGameLoader<'a>,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: BlockPosition,
  ) {
    if Some(block_position) != self.last_position {
      self.last_position = Some(block_position);

      let (want_loaded_vec, want_loaded_set) =
        SurroundingsLoader::wanted_blocks(timers, &block_position, self.load_distance);

      timers.time("update.update_queues.load_queue", || {
        for block_position in self.load_queue.iter() {
          terrain_game_loader.unmark_wanted(physics, block_position);
        }
        self.load_queue.clear();
        for block_position in want_loaded_vec.iter() {
          if terrain_game_loader.mark_wanted(id_allocator, physics, block_position) {
            self.load_queue.push_back(*block_position);
          }
        }
      });

      timers.time("update.update_queues.unload_queue", || {
        self.unload_queue.clear();
        for block_position in terrain_game_loader.loaded.iter() {
          let is_needed = want_loaded_set.contains(block_position);
          if !is_needed {
            self.unload_queue.push_back(*block_position);
          }
        }
      });
    }
  }

  #[inline]
  // Get the set of all blocks we want loaded around a given position.
  // Produces a Vec because order is important, and a HashSet for quick membership tests.
  fn wanted_blocks(
    timers: &TimerSet,
    position: &BlockPosition,
    load_distance: i32,
  ) -> (Vec<BlockPosition>, HashSet<BlockPosition>) {
    timers.time("update.update_queues.want_loaded", || {
      let mut want_loaded_vec = Vec::new();
      let mut want_loaded_set = HashSet::new();

      want_loaded_vec.push(*position);
      want_loaded_set.insert(*position);

      for radius in range_inclusive(1, load_distance) {
        let blocks_at_radius = cube_shell(position, radius);
        want_loaded_vec.push_all(blocks_at_radius.as_slice());
        for position in blocks_at_radius.into_iter() {
          want_loaded_set.insert(position);
        }
      }

      (want_loaded_vec, want_loaded_set)
    })
  }

  // Load some blocks. Prioritizes unloading unneeded ones over loading new ones.
  #[inline]
  fn load_some<'a>(
    &mut self,
    timers: &TimerSet,
    gl: &mut GLContext,
    terrain_game_loader: &mut TerrainGameLoader<'a>,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
  ) {
    let mut budget = BLOCK_UPDATE_BUDGET;
    while budget > 0 {
      match self.unload_queue.pop_front() {
        None =>
          match self.load_queue.pop_front() {
            None => {
              break;
            },
            Some(block_position) => {
              if terrain_game_loader.load(
                timers,
                gl,
                id_allocator,
                physics,
                &block_position,
              ) {
                budget -= BLOCK_LOAD_COST;
              }
            },
          },
        Some(block_position) => {
          timers.time("update.load_some.unload", || {
            if terrain_game_loader.unload(timers, gl, physics, &block_position) {
              budget -= BLOCK_UNLOAD_COST;
            }
          })
        }
      }
    }
  }
}

#[test]
fn shell_ordering() {
  fn radius_between(p1: &BlockPosition, p2: &BlockPosition) -> i32 {
    let dx = (p1.as_pnt().x - p2.as_pnt().x).abs();
    let dy = (p1.as_pnt().y - p2.as_pnt().y).abs();
    let dz = (p1.as_pnt().z - p2.as_pnt().z).abs();
    max(max(dx, dy), dz)
  }

  let mut loader = SurroundingsLoader::new(1);
  let timers = TimerSet::new();
  let mut id_allocator = IdAllocator::new();
  let mut physics = Physics::new(AABB::new(Pnt3::new(-128.0, -128.0, -128.0), Pnt3::new(128.0, 128.0, 128.0)));
  let position = BlockPosition::new(1, -4, 7);
  loader.update_queues(&timers, &mut id_allocator, &mut physics, position);
  let mut load_positions = loader.load_queue.into_iter();

  // The load queue should contain cube shells in increasing order of radius.
  for radius in range_inclusive(0, loader.load_distance) {
    for _ in range(0, cube_shell_area(radius)) {
      let load_position = load_positions.next();
      // The next load position should be in the cube shell of the given radius, relative to the center position.
      assert_eq!(radius_between(&position, &load_position.unwrap()), radius);
    }
  }

  // The load queue should be exactly the shells specified above, and nothing else.
  assert!(load_positions.next().is_none());
}
