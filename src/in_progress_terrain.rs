use id_allocator::IdAllocator;
use nalgebra::Vec3;
use ncollide::bounding_volume::AABB;
use physics::Physics;
use state::EntityId;
use std::collections::hash_map::{HashMap, Entry};
use terrain_block::{TerrainBlock, BlockPosition, BLOCK_WIDTH};

pub struct InProgressTerrain {
  pub blocks: HashMap<BlockPosition, EntityId>,
}

impl InProgressTerrain {
  pub fn new() -> InProgressTerrain {
    InProgressTerrain {
      blocks: HashMap::new(),
    }
  }

  /// Mark a block as in progress.
  pub fn insert(
    &mut self,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: &BlockPosition,
  ) {
    match self.blocks.entry(block_position.clone()) {
      Entry::Occupied(_) => {},
      Entry::Vacant(entry) => {
        let id = id_allocator.allocate();
        entry.set(id);

        let low_corner = TerrainBlock::to_world_position(block_position);
        let block_span = Vec3::new(BLOCK_WIDTH as f32, BLOCK_WIDTH as f32, BLOCK_WIDTH as f32);
        let bounds = AABB::new(low_corner, low_corner + block_span);
        physics.insert_misc(id, &bounds);
      }
    }
  }

  /// Unmark an in-progress block, either because loading is done, or the block was unloaded.
  pub fn remove(
    &mut self,
    physics: &mut Physics,
    block_position: &BlockPosition,
  ) {
    match self.blocks.remove(block_position) {
      None => {},
      Some(id) => {
        physics.remove_misc(id);
      },
    }
  }
}
