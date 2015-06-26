#![feature(range_inclusive)]
#![feature(cmp_partial)]
#![feature(iter_cmp)]

extern crate cgmath;
extern crate common;
#[macro_use]
extern crate log;
extern crate noise;
extern crate num;
extern crate stopwatch;

mod generate;
mod heightmap;
mod raycast;

pub mod voxel;
pub mod voxel_tree;

pub use noise::Seed;

use std::collections::hash_map::HashMap;
use std::collections::hash_set::HashSet;
use std::iter::range_inclusive;
use std::sync::Mutex;
use stopwatch::TimerSet;

use common::block_position::BlockPosition;
use common::entity::EntityId;
use common::id_allocator::IdAllocator;
use common::lod::LODIndex;
use common::terrain_block;
use common::terrain_block::TerrainBlock;

use heightmap::HeightMap;
use voxel::{Voxel, Fracu8, Fraci8};
use voxel_tree::VoxelTree;

pub const AMPLITUDE: f64 = 64.0;
pub const FREQUENCY: f64 = 1.0 / 64.0;
pub const PERSISTENCE: f64 = 1.0 / 16.0;
pub const LACUNARITY: f64 = 8.0;
pub const OCTAVES: usize = 2;

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
      heightmap: HeightMap::new(terrain_seed, OCTAVES, FREQUENCY, PERSISTENCE, LACUNARITY),
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

  pub fn remove_voxel<F>(
    &mut self,
    timers: &TimerSet,
    id_allocator: &Mutex<IdAllocator<EntityId>>,
    bounds: &voxel::Bounds,
    mut block_changed: F,
  )
    where F: FnMut(&TerrainBlock, &BlockPosition, LODIndex),
  {
   debug!("remove {:?}", bounds);

    match self.voxels.get_mut(bounds) {
      None => {
        return;
      },
      Some(voxel) => {
        *voxel = Voxel::Volume(false);
      },
    }

    // Ensure all the neighbors are populated.
    // TODO: Search for all these voxels in a single tree search.
    for &dx in [-1, 0, 1].iter() {
    for &dy in [-1, 0, 1].iter() {
    for &dz in [-1, 0, 1].iter() {
      let (x, y, z) = (bounds.x + dx, bounds.y + dy, bounds.z + dz);
      debug!("adjacent {:?} {:?} {:?}", x, y, z);

      let bounds = voxel::Bounds::new(x, y, z, bounds.lg_size);
      let voxel = self.voxels.get_mut_or_create(&bounds);
      debug!("voxel {:?}", *voxel);

      match voxel {
        &mut voxel_tree::TreeBody::Leaf(_) => {},
        _ => {
          *voxel =
            voxel_tree::TreeBody::Leaf(generate::generate_voxel(
              timers,
              &self.heightmap,
              &bounds,
            ));

          debug!("voxel changing to {:?}", *voxel);
        },
      }
      match voxel {
        &mut voxel_tree::TreeBody::Leaf(Voxel::Volume(true)) => {
          let half = Fracu8::of(0x80);
          let surface =
            voxel::SurfaceVoxel {
              inner_vertex: voxel::Vertex { x: half, y: half, z: half },
              // TODO: Make real normals.
              normal: voxel::Normal { x: Fraci8::of(0), y: Fraci8::of(0x7F), z: Fraci8::of(0) },
              corner_inside_surface: true,
            };
          *voxel = voxel_tree::TreeBody::Leaf(Voxel::Surface(surface));

          debug!("voxel detailing to {:?}", *voxel);
        },
        _ => {},
      }
    }}}

    // Remove all the corners.
    // TODO: Search for all these voxels in a single tree search.
    for &dx in [0, 1].iter() {
    for &dy in [0, 1].iter() {
    for &dz in [0, 1].iter() {
      let (x, y, z) = (bounds.x + dx, bounds.y + dy, bounds.z + dz);
      debug!("corner {:?} {:?} {:?}", x, y, z);

      let bounds = voxel::Bounds::new(x, y, z, bounds.lg_size);
      let voxel = self.voxels.get_mut(&bounds);

      debug!("voxel {:?}", voxel.as_ref().map(|&&mut v| v));
      match voxel {
        Some(&mut Voxel::Surface(ref mut voxel)) => {
          voxel.corner_inside_surface = false;
        },
        _ => {},
      }
    }}}

    // TODO: Consider regenerating the TerrainBlocks for the adjacent voxels too.

    // lg(number of voxels in a block)
    let lg_scale = terrain_block::LG_WIDTH - bounds.lg_size;

    let mut positions = HashSet::new();
    for &dx in [-1, 0, 1].iter() {
    for &dy in [-1, 0, 1].iter() {
    for &dz in [-1, 0, 1].iter() {
      let (x, y, z) = (bounds.x + dx, bounds.y + dy, bounds.z + dz);
      let bounds = voxel::Bounds::new(x, y, z, bounds.lg_size);
      let position =
        BlockPosition::new(bounds.x >> lg_scale, bounds.y >> lg_scale, bounds.z >> lg_scale);
      positions.insert(position);
    }}}

    for position in positions.into_iter() {
      let lod_index =
        terrain_block::LG_SAMPLE_SIZE.iter()
        .position(|&x| x == bounds.lg_size)
        .unwrap()
      ;
      let lod_index = LODIndex(lod_index as u32);

      let mip_mesh = self.all_blocks.get_mut(&position);
      let mesh = mip_mesh.get_mut(lod_index.0 as usize);
      *mesh = Some(
        generate::generate_block(
          timers,
          id_allocator,
          &self.heightmap,
          &mut self.voxels,
          &position,
          lod_index,
        )
      );

      block_changed(mesh.as_ref().unwrap(), &position, lod_index);
    }
  }
}
