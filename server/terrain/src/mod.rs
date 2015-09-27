//! This crate contains the terrain data structures and generation.

#![allow(let_and_return)]
#![allow(match_ref_pats)]
#![allow(type_complexity)]
#![deny(missing_docs)]
#![deny(warnings)]

#![feature(main)]
#![feature(plugin)]
#![feature(range_inclusive)]
#![feature(test)]
#![feature(unboxed_closures)]
#![feature(vec_push_all)]

#![plugin(clippy)]

extern crate cgmath;
extern crate common;
#[macro_use]
extern crate log;
extern crate noise;
extern crate rand;
extern crate stopwatch;
extern crate test;
extern crate time;
extern crate voxel as voxel_base;

mod generate;
pub mod biome;

pub mod tree;

pub use noise::Seed;

use cgmath::Aabb;
use std::collections::hash_map::HashMap;
use std::iter::range_inclusive;
use std::sync::Mutex;

use common::block_position::BlockPosition;
use common::entity::EntityId;
use common::id_allocator::IdAllocator;
use common::lod::LODIndex;
use common::terrain_block;
use common::terrain_block::TerrainBlock;

/// Voxel implementation for terrain
pub mod voxel {
  pub use voxel_base::impls::surface_vertex::*;

  #[derive(Debug, Copy, Clone, PartialEq, Eq)]
  #[allow(missing_docs)]
  /// Terrain materials
  pub enum Material {
    Empty = 0,
    Terrain = 1,
    Bark = 2,
    Leaves = 3,
    Stone = 4,
  }

  #[allow(missing_docs)]
  pub mod tree {
    use voxel_base;

    pub use voxel_base::tree::TreeBody::*;
    pub type T = voxel_base::tree::T<super::T<super::Material>>;
    pub type TreeBody = voxel_base::tree::TreeBody<super::T<super::Material>>;
    pub type Branches = voxel_base::tree::Branches<super::T<super::Material>>;
  }
}

/// Terrain mesh at multiple LODs.
pub struct MipMesh {
  #[allow(missing_docs)]
  pub lods: Vec<Option<TerrainBlock>>,
}

impl MipMesh {
  #[allow(missing_docs)]
  pub fn get_mut(&mut self, i: usize) -> &mut Option<TerrainBlock> {
    for _ in range_inclusive(self.lods.len(), i) {
      self.lods.push(None);
    }
    self.lods.get_mut(i).unwrap()
  }
}

/// Map block positions to the block mesh at various LODs.
pub struct MipMeshMap(pub HashMap<BlockPosition, MipMesh>);

impl MipMeshMap {
  #[allow(missing_docs)]
  pub fn new() -> MipMeshMap {
    MipMeshMap(HashMap::new())
  }

  #[allow(missing_docs)]
  pub fn get<'a>(&'a mut self, position: &BlockPosition) -> Option<&'a MipMesh> {
    self.0.get(position)
  }

  #[allow(missing_docs)]
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
#[allow(missing_docs)]
pub struct Terrain {
  pub mosaic: biome::hills::T,
  // all the blocks that have ever been created.
  pub all_blocks: MipMeshMap,
  pub voxels: voxel::tree::T,
}

impl Terrain {
  #[allow(missing_docs)]
  pub fn new(terrain_seed: Seed) -> Terrain {
    Terrain {
      mosaic: biome::hills::new(terrain_seed),
      all_blocks: MipMeshMap::new(),
      voxels: voxel::tree::T::new(),
    }
  }

  /// Load the block of terrain at a given position.
  // TODO: Allow this to be performed in such a way that self is only briefly locked.
  pub fn load<'a>(
    &'a mut self,
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
          id_allocator,
          &self.mosaic,
          &mut self.voxels,
          position,
          lod_index,
        )
      );
    }
    mesh.as_ref().unwrap()
  }

  /// Apply a voxel brush to the terrain.
  pub fn brush<F, Mosaic>(
    &mut self,
    id_allocator: &Mutex<IdAllocator<EntityId>>,
    brush: &voxel_base::brush::T<Mosaic>,
    mut block_changed: F,
  ) where
    F: FnMut(&TerrainBlock, &BlockPosition, LODIndex),
    Mosaic: voxel_base::mosaic::T<Material=voxel::Material>,
  {
    macro_rules! voxel_range(($d:ident, $scale:expr) => {{
      let low = brush.bounds.min().$d >> $scale;
      let high = brush.bounds.max().$d >> $scale;
      range_inclusive(low, high)
    }});

    // Make sure that all the voxels this brush might touch are generated; if they're not generated
    // now, the brush might "expose" them, the mesh extraction phase will generate them, and there
    // may be inconsistencies between the brush-altered voxels and the newly-generated ones.
    for &lg_size in &terrain_block::LG_SAMPLE_SIZE {
      for x in voxel_range!(x, lg_size) {
      for y in voxel_range!(y, lg_size) {
      for z in voxel_range!(z, lg_size) {
        let bounds = voxel_base::bounds::new(x, y, z, lg_size);
        let voxel = self.voxels.get_mut_or_create(&bounds);
        match voxel {
          &mut voxel::tree::Empty => {
            *voxel =
              voxel::tree::TreeBody::leaf(
                Some(voxel::unwrap(voxel::of_field(&self.mosaic, &bounds)))
              );
          },
          &mut voxel::tree::Branch { ref mut data, branches: _ } => {
            match data {
              &mut None => {
                *data =
                  Some(voxel::unwrap(voxel::of_field(&self.mosaic, &bounds)));
              },
              &mut Some(_) => {},
            }
          },
        }
      }}}
    }

    self.voxels.brush(brush);

    macro_rules! block_range(($d:ident) => {{
      let low = brush.bounds.min().$d >> terrain_block::LG_WIDTH;
      let high = brush.bounds.max().$d >> terrain_block::LG_WIDTH;
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
                id_allocator,
                &self.mosaic,
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
