use id_allocator::IdAllocator;
use noise::source::Perlin;
use state::EntityId;
use std::collections::hash_map::{HashMap, Entry};
use std::mem;
use stopwatch::TimerSet;
use terrain_block::{TerrainBlock, BlockPosition};

pub const AMPLITUDE: f32 = 256.0;
pub const FREQUENCY: f64 = 1.0 / 64.0;
pub const PERSISTENCE: f64 = 1.0 / 8.0;
pub const LACUNARITY: f64 = 8.0;
pub const OCTAVES: uint = 8;

pub const LOD_QUALITY: [uint, ..4] = [48, 16, 4, 2];

#[deriving(Show, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TerrainType {
  Grass,
  Dirt,
  Stone,
}

pub struct TerrainMipMesh {
  pub lods: [TerrainBlock, ..4],
}

/// This struct contains and lazily generates the world's terrain.
pub struct Terrain {
  // this is used for generating new blocks.
  pub heightmap: Perlin,
  // all the blocks that have ever been created.
  pub all_blocks: HashMap<BlockPosition, TerrainMipMesh>,
}

impl Terrain {
  pub fn new() -> Terrain {
    Terrain {
      heightmap:
        Perlin::new()
        .seed(0)
        .frequency(FREQUENCY)
        .persistence(PERSISTENCE)
        .lacunarity(LACUNARITY)
        .octaves(OCTAVES),
      all_blocks: HashMap::new(),
    }
  }

  #[inline]
  pub unsafe fn load<'a>(
    &'a mut self,
    timers: &TimerSet,
    id_allocator: &mut IdAllocator<EntityId>,
    position: &BlockPosition,
    lod: uint,
  ) -> &'a TerrainBlock {
    let mip_mesh =
      match self.all_blocks.entry(*position) {
        Entry::Occupied(entry) => {
          // Fudge the lifetime bounds
          mem::transmute(entry.get())
        },
        Entry::Vacant(entry) => {
          let heightmap = &self.heightmap;
          let generate = |lod| {
            TerrainBlock::generate(
              timers,
              id_allocator,
              heightmap,
              position,
              LOD_QUALITY[lod],
            )
          };
          let mip_mesh =
            TerrainMipMesh {
              lods: [generate(0), generate(1), generate(2), generate(3)],
            };
          let mip_mesh: &TerrainMipMesh = entry.set(mip_mesh);
          mip_mesh
        },
      };
    &mip_mesh.lods[lod]
  }
}
