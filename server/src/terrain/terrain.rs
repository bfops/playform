use cgmath::Vector3;
use noise::Seed;
use std::collections::hash_map::{HashMap, Entry};
use std::iter::range_inclusive;
use std::sync::Mutex;

use common::block_position::BlockPosition;
use common::entity::EntityId;
use common::id_allocator::IdAllocator;
use common::lod::LODIndex;
use common::stopwatch::TimerSet;
use common::terrain_block::TerrainBlock;

use terrain::heightmap::HeightMap;
use terrain::terrain_gen;
use voxel_tree::VoxelTree;

pub const AMPLITUDE: f64 = 64.0;
pub const FREQUENCY: f64 = 1.0 / 64.0;
pub const PERSISTENCE: f64 = 1.0 / 16.0;
pub const LACUNARITY: f64 = 8.0;
pub const OCTAVES: usize = 2;

#[derive(Copy)]
pub struct Voxel {
  pub edge_crosses: EdgeCrosses,
  pub vertex: Vector3<Frac8>,
  // When crossing an edge aligned with a given axis,
  // should the surface dot positive with the positive axis?
  pub facing: [bool; 3],
}

/// Tells you whether and where the surface crossed an edge of a cubic voxel.
#[derive(Debug, Copy)]
pub struct EdgeCrosses {
  // x_edges[y][z]
  pub x_edges: [[bool; 2]; 2],
  // y_edges[x][z]
  pub y_edges: [[bool; 2]; 2],
  // z_edges[x][y]
  pub z_edges: [[bool; 2]; 2],
}

#[derive(Copy)]
pub struct Frac8 {
  // The denominator is 1 << 8.
  pub numerator: u8,
}

impl Frac8 {
  pub fn of(numerator: u8) -> Frac8 {
    Frac8 {
      numerator: numerator,
    }
  }
}

#[test]
fn small_voxel() {
  use std::mem;

  // Check that the leaf does not increase the size of TreeBody on 64-bit systems.

  let max_ptr_size = mem::size_of::<u64>();
  println!("size_of::<EdgeCrosses>() = {}", mem::size_of::<EdgeCrosses>());
  println!("size_of::<Voxel>() = {}", mem::size_of::<Voxel>());
  assert!(mem::size_of::<Voxel>() <= max_ptr_size);
}

pub struct TerrainMipMesh {
  pub lods: Vec<Option<TerrainBlock>>,
}

/// This struct contains and lazily generates the world's terrain.
pub struct Terrain {
  pub heightmap: HeightMap,
  // all the blocks that have ever been created.
  pub all_blocks: HashMap<BlockPosition, TerrainMipMesh>,
  pub voxels: VoxelTree<Voxel>,
}

impl Terrain {
  pub fn new(terrain_seed: Seed) -> Terrain {
    Terrain {
      heightmap: HeightMap::new(terrain_seed, OCTAVES, FREQUENCY, PERSISTENCE, LACUNARITY, AMPLITUDE),
      all_blocks: HashMap::new(),
      voxels: VoxelTree::new(),
    }
  }

  // TODO: Allow this to be performed in such a way that self is only briefly locked.
  pub fn load<F, T>(
    &mut self,
    timers: &TimerSet,
    id_allocator: &Mutex<IdAllocator<EntityId>>,
    position: &BlockPosition,
    lod_index: LODIndex,
    f: F,
  ) -> T
    where F: FnOnce(&TerrainBlock) -> T
  {
    macro_rules! load_lod(
      ($mip_mesh: expr) => ({
        for _ in range_inclusive($mip_mesh.lods.len(), lod_index.0 as usize) {
          $mip_mesh.lods.push(None);
        }
        let mesh = $mip_mesh.lods.get_mut(lod_index.0 as usize).unwrap();
        if mesh.is_none() {
          *mesh = Some(
            terrain_gen::generate_block(
              timers,
              id_allocator,
              &self.heightmap,
              &mut self.voxels,
              position,
              lod_index,
            )
          );
        }

        f(mesh.as_ref().unwrap())
      })
    );

    match self.all_blocks.entry(*position) {
      Entry::Occupied(mut entry) => {
        load_lod!(entry.get_mut())
      },
      Entry::Vacant(entry) => {
        let r = entry.insert(
          TerrainMipMesh {
            lods: Vec::new(),
          }
        );
        load_lod!(r)
      },
    }
  }
}
