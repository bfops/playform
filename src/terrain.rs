use id_allocator::IdAllocator;
use noise::{Seed, Brownian2, perlin2, Point2};
use state::EntityId;
use std::collections::hash_map::{HashMap, Entry};
use std::mem;
use stopwatch::TimerSet;
use terrain_block::{TerrainBlock, BlockPosition};

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

pub struct TerrainMipMesh {
  pub lods: Vec<Option<TerrainBlock>>,
}

/// This struct contains and lazily generates the world's terrain.
pub struct Terrain {
  pub seed: Seed,
  // this is used for generating new blocks.
  pub heightmap: Brownian2<f64, fn (&Seed, &Point2<f64>) -> f64>,
  // all the blocks that have ever been created.
  pub all_blocks: HashMap<BlockPosition, TerrainMipMesh>,
}

impl Terrain {
  pub fn new(seed: Seed) -> Terrain {
    let perlin2: fn(&Seed, &Point2<f64>) -> f64 = perlin2;
    Terrain {
      seed: seed,
      heightmap:
        Brownian2::new(perlin2, OCTAVES)
        .frequency(FREQUENCY)
        .persistence(PERSISTENCE)
        .lacunarity(LACUNARITY),
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
          &self.seed,
        )
      );
    }
    
    mesh.as_ref().unwrap()
  }
}
