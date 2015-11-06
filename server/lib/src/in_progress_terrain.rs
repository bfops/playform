use cgmath::{Aabb3};
use std::collections::hash_map::{HashMap, Entry};
use std::sync::Mutex;
use voxel_data;

use common::entity_id;
use common::id_allocator;

use physics::Physics;

// TODO: Rename this to something more memorable.
pub struct T {
  pub blocks: HashMap<voxel_data::bounds::T, entity_id::T>,
}

impl T {
  pub fn new() -> T {
    T {
      blocks: HashMap::new(),
    }
  }

  /// Mark a block as in-progress by making it solid.
  pub fn insert(
    &mut self,
    id_allocator: &Mutex<id_allocator::T<entity_id::T>>,
    physics: &Mutex<Physics>,
    block_position: &voxel_data::bounds::T,
  ) -> bool {
    match self.blocks.entry(*block_position) {
      Entry::Occupied(_) => {
        warn!("Re-inserting {:?}", block_position);
        false
      },
      Entry::Vacant(entry) => {
        let id = id_allocator.lock().unwrap().allocate();
        entry.insert(id);

        let (low, high) = block_position.corners();
        physics.lock().unwrap().insert_misc(id, Aabb3::new(low, high));
        true
      }
    }
  }

  /// Unmark an in-progress block, either because loading is done, or the block was unloaded.
  pub fn remove(
    &mut self,
    physics: &Mutex<Physics>,
    block_position: &voxel_data::bounds::T,
  ) -> bool {
    self.blocks.remove(block_position)
      .map(|id| physics.lock().unwrap().remove_misc(id)).is_some()
  }
}
