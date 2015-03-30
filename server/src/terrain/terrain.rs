use bit_svo::{VoxelTree, VoxelBounds};
use cgmath::{Point, Point3, EuclideanVector, Vector3};
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

pub const AMPLITUDE: f64 = 64.0;
pub const FREQUENCY: f64 = 1.0 / 64.0;
pub const PERSISTENCE: f64 = 1.0 / 16.0;
pub const LACUNARITY: f64 = 8.0;
pub const OCTAVES: usize = 2;

#[derive(Debug, Copy)]
pub enum Voxel {
  Empty,
  Surface(SurfaceVoxel),
}

#[derive(Debug, Copy)]
pub struct SurfaceVoxel {
  pub vertex: VoxelVertex,
  pub normal: VoxelNormal,

  // Each voxel contains edge information about the edges that intersect the
  // lowest corner of the cube. Assuming the grid is effectively infinite, no
  // edges will be left out.

  pub x_edge: Edge,
  pub y_edge: Edge,
  pub z_edge: Edge,
}

#[derive(Debug, Copy)]
pub struct VoxelVertex {
  pub x: Fracu8,
  pub y: Fracu8,
  pub z: Fracu8,
}

impl VoxelVertex {
  pub fn to_world_vertex(&self, parent: VoxelBounds) -> Point3<f32> {
    // Relative position of the vertex.
    let local =
      Vector3::new(
        self.x.numerator as f32 / 256.0,
        self.y.numerator as f32 / 256.0,
        self.z.numerator as f32 / 256.0,
      );
    let fparent = Point3::new(parent.x as f32, parent.y as f32, parent.z as f32);
    fparent.add_v(&local).mul_s(parent.size())
  }
}

#[derive(Debug, Copy)]
pub struct VoxelNormal {
  pub x: Fraci8,
  pub y: Fraci8,
  pub z: Fraci8,
}

impl VoxelNormal {
  pub fn to_world_normal(&self) -> Vector3<f32> {
    Vector3::new(self.x.to_f32(), self.y.to_f32(), self.z.to_f32()).normalize()
  }
}

/// Tells you whether and where the surface crossed an edge of a cubic voxel.
#[derive(Debug, Copy)]
pub struct Edge {
  pub is_crossed: bool,
  // If this is true, the edge moves into the volume as its coordinates increase.
  pub direction: bool,
}

/// Express a `[0,1)` fraction using a `u8`.
#[derive(Debug, Copy)]
pub struct Fracu8 {
  // The denominator is 1 << 8.
  pub numerator: u8,
}

impl Fracu8 {
  pub fn of(numerator: u8) -> Fracu8 {
    Fracu8 {
      numerator: numerator,
    }
  }
}

/// Express a `[0,1)` fraction using a `i8`.
#[derive(Debug, Copy)]
pub struct Fraci8 {
  // The denominator is 1 << 8.
  pub numerator: i8,
}

impl Fraci8 {
  pub fn of(numerator: i8) -> Fraci8 {
    Fraci8 {
      numerator: numerator,
    }
  }

  pub fn to_f32(&self) -> f32 {
    self.numerator as f32 / 128.0
  }
}

#[test]
fn small_voxel() {
  use std::mem;

  // Check that the leaf does not increase the size of TreeBody on 64-bit systems.

  let max_ptr_size = mem::size_of::<u64>();
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
      heightmap: HeightMap::new(terrain_seed, OCTAVES, FREQUENCY, PERSISTENCE, LACUNARITY),
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
