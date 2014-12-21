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

pub const LOAD_DISTANCE: int = 10;

pub const BLOCK_UPDATE_BUDGET: int = 16;
pub const BLOCK_LOAD_COST: int = 2;
pub const BLOCK_UNLOAD_COST: int = 1;

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
    physics: &mut Physics<EntityId>,
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

      // TODO: Order this in rings around block_position.
      let mut want_loaded = Vec::new();
      let mut want_loaded_set = HashSet::new();

      timers.time("update.update_queues.want_loaded", || {
        want_loaded.push(block_position);
        want_loaded_set.insert(block_position);

        for radius in range_inclusive(1, LOAD_DISTANCE) {
          let blocks_at_radius = cube_shell(block_position, radius);
          want_loaded.push_all(blocks_at_radius.as_slice());
          for block_position in blocks_at_radius.into_iter() {
            want_loaded_set.insert(block_position);
          }
        }
      });

      self.load_queue.clear();

      timers.time("update.update_queues.unload_queue", || {
        for block_position in self.want_loaded.iter() {
          let is_loaded = self.loaded.contains(block_position);
          if is_loaded {
            let is_needed = want_loaded_set.contains(block_position);
            if !is_needed {
              self.unload_queue.push_back(*block_position);
            }
          }
        }
      });

      timers.time("update.update_queues.load_queue", || {
        for block_position in want_loaded.iter() {
          let is_loaded = self.loaded.contains(block_position);
          if !is_loaded {
            self.load_queue.push_back(*block_position);
          }
        }
      });

      self.want_loaded = want_loaded_set;
    }
  }

  #[inline]
  fn load_some(
    &mut self,
    timers: &TimerSet,
    gl: &mut GLContext,
    terrain_buffers: &mut TerrainVRAMBuffers<'a>,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics<EntityId>,
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
                    physics.insert(id, bounds);
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
              physics.remove(*id);
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
