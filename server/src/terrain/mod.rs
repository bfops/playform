pub mod brush;
mod field;
mod generate;
mod heightmap;
mod raycast;

pub mod voxel;
pub mod voxel_tree;

pub use noise::Seed;

use std::collections::hash_map::HashMap;
use std::iter::range_inclusive;
use std::sync::Mutex;
use stopwatch::TimerSet;

use common::block_position::BlockPosition;
use common::entity::EntityId;
use common::id_allocator::IdAllocator;
use common::lod::LODIndex;
use common::terrain_block;
use common::terrain_block::TerrainBlock;

use self::heightmap::HeightMap;
use self::voxel_tree::VoxelTree;

pub struct MipMesh {
  pub lods: Vec<Option<TerrainBlock>>,
}

impl MipMesh {
  pub fn get_mut<'a>(&'a mut self, i: usize) -> &'a mut Option<TerrainBlock> {
    for _ in range_inclusive(self.lods.len(), i) {
      self.lods.push(None);
    }
    self.lods.get_mut(i).unwrap()
  }
}

pub struct MipMeshMap(pub HashMap<BlockPosition, MipMesh>);

impl MipMeshMap {
  pub fn new() -> MipMeshMap {
    MipMeshMap(HashMap::new())
  }

  pub fn get<'a>(&'a mut self, position: &BlockPosition) -> Option<&'a MipMesh> {
    self.0.get(position)
  }

  pub fn get_mut<'a>(&'a mut self, position: &BlockPosition) -> &'a mut MipMesh {
    self.0
      .entry(*position)
      .or_insert_with(|| {
        MipMesh {
          lods: Vec::new(),
        }
      })
  }
}

/// This struct contains and lazily generates the world's terrain.
pub struct Terrain {
  pub heightmap: HeightMap,
  // all the blocks that have ever been created.
  pub all_blocks: MipMeshMap,
  pub voxels: VoxelTree,
}

impl Terrain {
  pub fn new(terrain_seed: Seed) -> Terrain {
    Terrain {
      heightmap: HeightMap::new(terrain_seed),
      all_blocks: MipMeshMap::new(),
      voxels: VoxelTree::new(),
    }
  }

  // TODO: Allow this to be performed in such a way that self is only briefly locked.
  pub fn load<'a>(
    &'a mut self,
    timers: &TimerSet,
    id_allocator: &Mutex<IdAllocator<EntityId>>,
    position: &BlockPosition,
    lod_index: LODIndex,
  ) -> &'a TerrainBlock
  {
    let mip_mesh = self.all_blocks.get_mut(position);
    let mesh = mip_mesh.get_mut(lod_index.0 as usize);
    if mesh.is_none() {
      *mesh = Some(
        generate::generate_block(
          timers,
          id_allocator,
          &self.heightmap,
          &mut self.voxels,
          position,
          lod_index,
        )
      );
    }
    mesh.as_ref().unwrap()
  }

  pub fn remove<F, Brush>(
    &mut self,
    timers: &TimerSet,
    id_allocator: &Mutex<IdAllocator<EntityId>>,
    brush: &Brush,
    brush_bounds: &brush::Bounds,
    mut block_changed: F,
  ) where
    F: FnMut(&TerrainBlock, &BlockPosition, LODIndex),
    Brush: brush::Brush,
  {
    self.voxels.remove(brush, brush_bounds);

    macro_rules! block_range(($d:ident) => {{
      let low = brush_bounds.low.$d >> terrain_block::LG_WIDTH;
      let high = brush_bounds.high.$d >> terrain_block::LG_WIDTH;
      range_inclusive(low, high)
    }});

    for x in block_range!(x) {
    for y in block_range!(y) {
    for z in block_range!(z) {
      let position = BlockPosition::new(x, y, z);
      let mip_mesh = self.all_blocks.get_mut(&position);

      for (i, mesh) in mip_mesh.lods.iter_mut().enumerate() {
        match mesh {
          &mut None => {},
          &mut Some(ref mut mesh) => {
            let lod_index = LODIndex(i as u32);
            *mesh =
              generate::generate_block(
                timers,
                id_allocator,
                &self.heightmap,
                &mut self.voxels,
                &position,
                lod_index,
              )
            ;

            block_changed(mesh, &position, lod_index);
          },
        }
      }
    }}}
  }
}
