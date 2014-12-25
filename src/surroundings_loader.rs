use cube_shell::cube_shell;
use id_allocator::IdAllocator;
use terrain_vram_buffers::TerrainVRAMBuffers;
use terrain::BlockPosition;
use terrain::Terrain;
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
use std::cmp::max;
#[cfg(test)]
use std::num::SignedInt;

pub const LOAD_DISTANCE: int = 10;

// values are approximately in microseconds, but they don't have to be.
pub const BLOCK_UPDATE_BUDGET: int = 5000;
pub const BLOCK_LOAD_COST: int = 500;
pub const BLOCK_UNLOAD_COST: int = 300;

/// Keep surroundings loaded around a given world position.
pub struct SurroundingsLoader<'a> {
  pub terrain: Terrain<'a>,

  pub load_queue: RingBuf<BlockPosition>,
  pub unload_queue: RingBuf<BlockPosition>,

  // the set of blocks that are currently loaded
  pub loaded: HashSet<BlockPosition>,
  // The set of all blocks we want loaded.
  pub want_loaded: HashSet<BlockPosition>,

  pub last_position: Option<BlockPosition>,
}

impl<'a> SurroundingsLoader<'a> {
  pub fn new() -> SurroundingsLoader<'a> {
    SurroundingsLoader {
      terrain: Terrain::new(),

      load_queue: RingBuf::new(),
      unload_queue: RingBuf::new(),

      loaded: HashSet::new(),
      want_loaded: HashSet::new(),

      last_position: None,
    }
  }

  pub fn update(
    &mut self,
    timers: &TimerSet,
    gl: &mut GLContext,
    terrain_buffers: &mut TerrainVRAMBuffers<'a>,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    position: BlockPosition,
  ) {
    timers.time("update.update_queues", || {
      self.update_queues(timers, position);
    });
    timers.time("update.load_some", || {
      self.load_some(timers, gl, terrain_buffers, id_allocator, physics);
    });
  }

  #[inline]
  fn update_queues(&mut self, timers: &TimerSet, block_position: BlockPosition) {
    if Some(block_position) != self.last_position {
      self.last_position = Some(block_position);

      let (want_loaded_vec, want_loaded_set) = SurroundingsLoader::wanted_blocks(timers, &block_position);

      timers.time("update.update_queues.load_queue", || {
        self.load_queue.clear();
        for block_position in want_loaded_vec.iter() {
          let is_loaded = self.loaded.contains(block_position);
          if !is_loaded {
            self.load_queue.push_back(*block_position);
          }
        }
      });

      timers.time("update.update_queues.unload_queue", || {
        self.unload_queue.clear();
        for block_position in self.loaded.iter() {
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
  fn wanted_blocks(timers: &TimerSet, position: &BlockPosition) -> (Vec<BlockPosition>, HashSet<BlockPosition>) {
    timers.time("update.update_queues.want_loaded", || {
      let mut want_loaded_vec = Vec::new();
      let mut want_loaded_set = HashSet::new();

      want_loaded_vec.push(position.clone());
      want_loaded_set.insert(position.clone());

      for radius in range_inclusive(1, LOAD_DISTANCE) {
        let blocks_at_radius = cube_shell(position, radius);
        for position in blocks_at_radius.into_iter() {
          want_loaded_vec.push(position);
          want_loaded_set.insert(position);
        }
      }

      (want_loaded_vec, want_loaded_set)
    })
  }

  #[inline]
  fn load_some(
    &mut self,
    timers: &TimerSet,
    gl: &mut GLContext,
    terrain_buffers: &mut TerrainVRAMBuffers<'a>,
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
              timers.time("update.load_some.load", || {
                let block = self.terrain.load(timers, id_allocator, &block_position);

                timers.time("update.load_some.load.physics", || {
                  for (&id, bounds) in block.bounds.iter() {
                    physics.insert_terrain(id, bounds);
                  }
                });

                timers.time("update.load_some.load.vram", || {
                  terrain_buffers.push(
                    gl,
                    block.vertices.as_slice(),
                    block.normals.as_slice(),
                    block.typs.as_slice(),
                    block.ids.as_slice(),
                  );
                });
              });

              self.loaded.insert(block_position);
              budget -= BLOCK_LOAD_COST;
            },
          },
        Some(block_position) => {
          timers.time("update.load_some.unload", || {
            let block = self.terrain.all_blocks.get(&block_position).unwrap();
            for id in block.ids.iter() {
              physics.remove_terrain(*id);
              terrain_buffers.swap_remove(gl, *id);
            }

            self.loaded.remove(&block_position);
            budget -= BLOCK_UNLOAD_COST;
          })
        }
      }
    }
  }
}

#[test]
fn shell_ordering() {
  fn radius_between(p1: &BlockPosition, p2: &BlockPosition) -> int {
    let dx = (p1.x - p2.x).abs();
    let dy = (p1.y - p2.y).abs();;
    let dz = (p1.z - p2.z).abs();;
    max(max(dx, dy), dz)
  }

  let mut loader = SurroundingsLoader::new();
  let timers = TimerSet::new();
  let position = Pnt3::new(1, -4, 7);
  loader.update_queues(&timers, position);
  let mut loader = loader.load_queue.into_iter();

  // The load queue should contain cube shells in increasing order of radius, up to LOAD_DISTANCE radius.
  for radius in range_inclusive(0, LOAD_DISTANCE) {
    for _ in range(0, cube_shell_area(radius)) {
      let load_position = loader.next();
      // The next load position should be in the cube shell of the given radius, relative to the center position.
      assert_eq!(radius_between(&position, &load_position.unwrap()), radius);
    }
  }

  // The load queue should be exactly the shells specified above, and nothing else.
  assert!(loader.next().is_none());
}
