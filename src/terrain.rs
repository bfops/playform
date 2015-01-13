use color::Color3;
use id_allocator::IdAllocator;
use noise::Seed;
use state::EntityId;
use std::collections::hash_map::{HashMap, Entry};
use std::mem;
use stopwatch::TimerSet;
use terrain_block::{TerrainBlock, BlockPosition};
use terrain_heightmap::HeightMap;

pub const AMPLITUDE: f64 = 256.0;
pub const FREQUENCY: f64 = 1.0 / 64.0;
pub const PERSISTENCE: f64 = 1.0 / 16.0;
pub const LACUNARITY: f64 = 8.0;
pub const OCTAVES: usize = 3;

// Quality is the number of times the noise function is sampled along each axis.
pub const LOD_QUALITY: [u32; 5] = [32, 16, 4, 2, 1];

#[derive(Show, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TerrainType {
  Grass,
  Dirt,
  Stone,
}

impl TerrainType {
  pub fn color(&self) -> Color3<f32> {
    match *self {
      TerrainType::Grass => Color3::of_rgb(0.0, 0.5, 0.0),
      TerrainType::Dirt => Color3::of_rgb(0.5, 0.4, 0.2),
      TerrainType::Stone => Color3::of_rgb(0.5, 0.5, 0.5),
    }
  }
}

pub struct TerrainMipMesh {
  pub lods: Vec<Option<TerrainBlock>>,
}

/// This struct contains and lazily generates the world's terrain.
pub struct Terrain {
  pub heightmap: HeightMap,
  // all the blocks that have ever been created.
  pub all_blocks: HashMap<BlockPosition, TerrainMipMesh>,
}

impl Terrain {
  pub fn new(terrain_seed: Seed) -> Terrain {
    Terrain {
      heightmap:
        HeightMap::new(terrain_seed, OCTAVES, FREQUENCY, PERSISTENCE, LACUNARITY, AMPLITUDE),
      all_blocks: HashMap::new(),
    }
  }

  pub unsafe fn load<'a>(
    &'a mut self,
    timers: &TimerSet,
    id_allocator: &mut IdAllocator<EntityId>,
    position: &'a BlockPosition,
    lod_index: u32,
  ) -> &'a TerrainBlock {
    let mip_mesh =
      match self.all_blocks.entry(*position) {
        Entry::Occupied(mut entry) => {
          // Fudge the lifetime bounds
          mem::transmute(entry.get_mut())
        },
        Entry::Vacant(entry) => {
          entry.insert(
            TerrainMipMesh {
              lods: range(0, LOD_QUALITY.len()).map(|_| None).collect(),
            }
          )
        },
      };

    let mesh = mip_mesh.lods.get_mut(lod_index as usize).unwrap();
    if mesh.is_none() {
      *mesh = Some(
        TerrainBlock::generate(
          timers,
          id_allocator,
          &self.heightmap,
          position,
          LOD_QUALITY[lod_index as usize],
        )
      );
    }
    
    mesh.as_ref().unwrap()
  }
}
