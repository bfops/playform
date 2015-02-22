use common::block_position::BlockPosition;
use common::entity::EntityId;
use common::id_allocator::IdAllocator;
use common::terrain_block::BLOCK_WIDTH;
use cgmath::{Aabb3, Point, Vector, Vector3};
use physics::Physics;
use std::collections::hash_map::{HashMap, Entry};
use std::sync::Mutex;

// TODO: Rename this to something more memorable.
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
    id_allocator: &Mutex<IdAllocator<EntityId>>,
    physics: &Mutex<Physics>,
    block_position: &BlockPosition,
  ) -> bool {
    match self.blocks.entry(*block_position) {
      Entry::Occupied(_) => {
        warn!("Re-inserting {:?}", block_position);
        false
      },
      Entry::Vacant(entry) => {
        let id = id_allocator.lock().unwrap().allocate();
        entry.insert(id);

        let low_corner = block_position.to_world_position();
        let block_span = Vector3::new(BLOCK_WIDTH as f32, BLOCK_WIDTH as f32, BLOCK_WIDTH as f32);
        physics.lock().unwrap().insert_misc(id, Aabb3::new(low_corner, low_corner.add_v(&block_span)));
        true
      }
    }
  }

  /// Unmark an in-progress block, either because loading is done, or the block was unloaded.
  pub fn remove(
    &mut self,
    physics: &Mutex<Physics>,
    block_position: &BlockPosition,
  ) -> bool {
    self.blocks.remove(block_position).map(|id| physics.lock().unwrap().remove_misc(id)).is_some()
  }
}
