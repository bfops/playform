use cgmath::{Aabb3};
use std::sync::Mutex;

use common::entity_id;
use common::fnv_map;
use common::id_allocator;
use common::voxel;

use physics::Physics;

// TODO: Rename this to something more memorable.
pub struct T {
  pub blocks: fnv_map::T<voxel::bounds::T, entity_id::T>,
}

impl T {
  pub fn new() -> T {
    T {
      blocks: fnv_map::new(),
    }
  }

  /// Mark a block as in-progress by making it solid.
  pub fn insert(
    &mut self,
    id_allocator: &Mutex<id_allocator::T<entity_id::T>>,
    physics: &Mutex<Physics>,
    block_position: &voxel::bounds::T,
  ) -> bool {
    match self.blocks.entry(*block_position) {
      fnv_map::Entry::Occupied(_) => {
        warn!("Re-inserting {:?}", block_position);
        false
      },
      fnv_map::Entry::Vacant(entry) => {
        let id = id_allocator::allocate(id_allocator);
        entry.insert(id);

        let (low, high) = block_position.corners();
        physics.lock().unwrap().insert_misc(id, &Aabb3::new(low, high));
        true
      }
    }
  }

  /// Unmark an in-progress block, either because loading is done, or the block was unloaded.
  pub fn remove(
    &mut self,
    physics: &Mutex<Physics>,
    block_position: &voxel::bounds::T,
  ) -> bool {
    self.blocks.remove(block_position)
      .map(|id| physics.lock().unwrap().remove_misc(id)).is_some()
  }
}
