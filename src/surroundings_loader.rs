use cube_shell::cube_shell;
use id_allocator::IdAllocator;
use in_progress_terrain::InProgressTerrain;
use terrain::Terrain;
use terrain_block::BlockPosition;
use terrain_vram_buffers;
use terrain_vram_buffers::TerrainVRAMBuffers;
use physics::Physics;
use state::EntityId;
use std::collections::HashSet;
use std::collections::RingBuf;
use std::iter::range_inclusive;
use std::num::Float;
use stopwatch::TimerSet;
use terrain_block::SAMPLES_PER_BLOCK;
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
pub const BLOCK_LOAD_COST: int = 400;
pub const BLOCK_UNLOAD_COST: int = 200;

pub const POLYGONS_PER_BLOCK: i32 = SAMPLES_PER_BLOCK as i32 * SAMPLES_PER_BLOCK as i32 * 4;

/// Keep surroundings loaded around a given world position.
pub struct SurroundingsLoader {
  pub terrain: Terrain,
  pub in_progress_terrain: InProgressTerrain,

  pub load_queue: RingBuf<BlockPosition>,
  pub unload_queue: RingBuf<BlockPosition>,

  pub load_distance: i32,

  // the set of blocks that are currently loaded
  pub loaded: HashSet<BlockPosition>,

  pub last_position: Option<BlockPosition>,
}

impl SurroundingsLoader {
  pub fn new(portion_of_polygon_budget: i32) -> SurroundingsLoader {
    assert!(portion_of_polygon_budget > 0);

    let block_budget =
      terrain_vram_buffers::POLYGON_BUDGET as i32
      / (portion_of_polygon_budget * POLYGONS_PER_BLOCK);
    // We'll have at most load_width^2 full blocks loaded.
    // This is because we generate flat terrain! If that changes, this changes!
    let load_width = (block_budget as f32).sqrt() as i32;
    let load_distance = (load_width - 1) / 2;

    SurroundingsLoader {
      terrain: Terrain::new(),
      in_progress_terrain: InProgressTerrain::new(),

      load_queue: RingBuf::new(),
      unload_queue: RingBuf::new(),

      load_distance: load_distance,

      loaded: HashSet::new(),

      last_position: None,
    }
  }

  pub fn update(
    &mut self,
    timers: &TimerSet,
    gl: &mut GLContext,
    terrain_buffers: &mut TerrainVRAMBuffers,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    position: BlockPosition,
  ) {
    timers.time("update.update_queues", || {
      self.update_queues(timers, id_allocator, physics, position);
    });
    timers.time("update.load_some", || {
      self.load_some(timers, gl, terrain_buffers, id_allocator, physics);
    });
  }

  #[inline]
  fn update_queues(
    &mut self,
    timers: &TimerSet,
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
          self.in_progress_terrain.remove(physics, block_position);
        }
        self.load_queue.clear();
        for block_position in want_loaded_vec.iter() {
          let is_loaded = self.loaded.contains(block_position);
          if !is_loaded {
            self.in_progress_terrain.insert(id_allocator, physics, block_position);
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
  fn load_some(
    &mut self,
    timers: &TimerSet,
    gl: &mut GLContext,
    terrain_buffers: &mut TerrainVRAMBuffers,
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
                let block = unsafe {
                  self.terrain.load(timers, id_allocator, &block_position)
                };

                timers.time("update.load_some.load.physics", || {
                  for (&id, bounds) in block.bounds.iter() {
                    physics.insert_terrain(id, bounds);
                  }
                });

                timers.time("update.load_some.load.vram", || {
                  terrain_buffers.push(
                    gl,
                    block.vertex_coordinates.as_slice(),
                    block.normals.as_slice(),
                    block.typs.as_slice(),
                    block.ids.as_slice(),
                  );
                });
              });

              self.in_progress_terrain.remove(physics, &block_position);
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
      println!("radius {}", radius);
      println!("load_position {}", load_position);
      // The next load position should be in the cube shell of the given radius, relative to the center position.
      assert_eq!(radius_between(&position, &load_position.unwrap()), radius);
    }
  }

  // The load queue should be exactly the shells specified above, and nothing else.
  assert!(load_positions.next().is_none());
}
