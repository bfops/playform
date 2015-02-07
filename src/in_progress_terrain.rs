use id_allocator::IdAllocator;
use nalgebra::Vec3;
use ncollide_entities::bounding_volume::AABB;
use physics::Physics;
use state::EntityId;
use std::collections::hash_map::{HashMap, Entry};
use terrain::terrain_block::{BlockPosition, BLOCK_WIDTH};

pub struct InProgressTerrain {
  pub blocks: HashMap<BlockPosition, EntityId>,
}

impl InProgressTerrain {
  pub fn new() -> InProgressTerrain {
    InProgressTerrain {
      blocks: HashMap::new(),
    }
  }

  /// Mark a block as in-progress by making it solid.
  pub fn insert(
    &mut self,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: &BlockPosition,
  ) -> bool {
    match self.blocks.entry(*block_position) {
      Entry::Occupied(_) => false,
      Entry::Vacant(entry) => {
        let id = id_allocator.allocate();
        entry.insert(id);

        let low_corner = block_position.to_world_position();
        let block_span = Vec3::new(BLOCK_WIDTH as f32, BLOCK_WIDTH as f32, BLOCK_WIDTH as f32);
        physics.insert_misc(id, AABB::new(low_corner, low_corner + block_span));
        true
      }
    }
  }

  /// Unmark an in-progress block, either because loading is done, or the block was unloaded.
  pub fn remove(
    &mut self,
    physics: &mut Physics,
    block_position: &BlockPosition,
  ) -> bool {
    self.blocks.remove(block_position).map(|id| physics.remove_misc(id)).is_some()
  }
}
