use cube_shell::cube_shell;
use id_allocator::IdAllocator;
use terrain_block::BlockPosition;
use terrain_game_loader::{TerrainGameLoader, LOD, OwnerId};
use physics::Physics;
use state::EntityId;
use std::cmp::max;
use std::num::Float;
use std::num::SignedInt;
use surroundings_iter::SurroundingsIter;
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
  pub lod_index: Box<FnMut(i32) -> uint + 'a>,

  pub max_load_distance: i32,
  pub to_load: Option<SurroundingsIter>,

  pub loaded_vec: Vec<BlockPosition>,
  pub next_unload_index: uint,

  pub solid_boundary: Vec<BlockPosition>,
}

impl<'a> SurroundingsLoader<'a> {
  pub fn new(
    id: OwnerId,
    max_load_distance: i32,
    lod_index: Box<FnMut(i32) -> uint + 'a>,
  ) -> SurroundingsLoader<'a> {
    assert!(max_load_distance >= 0);

    SurroundingsLoader {
      id: id,
      last_position: None,
      lod_index: lod_index,

      to_load: None,
      max_load_distance: max_load_distance,

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
    let position_changed = Some(position) != self.last_position;
    if position_changed {
      self.last_position.map(
        |last_position| {
          let mut iter =
            self.solid_boundary
              .iter()
              .map(|&dp| last_position + dp.as_pnt().to_vec());
          for solid_block in iter {
            terrain_game_loader.remove_placeholder(physics, &solid_block, self.id);
          }
        });

      // This will make some of the remove_placeholders calls redundant. Fix?
      for solid_block in self.solid_boundary.iter().map(|&dp| position + dp.as_pnt().to_vec()) {
        terrain_game_loader.insert_placeholder(id_allocator, physics, &solid_block, self.id);
      }

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
          terrain_game_loader.unload(timers, gl, id_allocator, physics, &block_position, self.id);
          self.loaded_vec.swap_remove(self.next_unload_index);
        } else {
          let lod_index = (self.lod_index)(distance);
          terrain_game_loader.decrease_lod(
            timers,
            gl,
            id_allocator,
            physics,
            &block_position,
            LOD::LodIndex(lod_index),
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

        let lod_index = (self.lod_index)(self.to_load.as_ref().unwrap().next_distance);

        terrain_game_loader.load(
          timers,
          gl,
          id_allocator,
          physics,
          &block_position,
          LOD::LodIndex(lod_index),
          self.id,
        );

        self.loaded_vec.push(block_position);
      }
    }
  }
}
