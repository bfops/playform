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

pub const LOD_QUALITY: [uint, ..5] = [48, 16, 4, 2, 1];

#[deriving(Show, Copy, Clone, PartialEq, Eq, Hash)]
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
    lod_index: uint,
  ) -> &'a TerrainBlock {
    let mip_mesh =
      match self.all_blocks.entry(*position) {
        Entry::Occupied(mut entry) => {
          // Fudge the lifetime bounds
          mem::transmute(entry.get_mut())
        },
        Entry::Vacant(entry) => {
          entry.set(
            TerrainMipMesh {
              lods: range(0, LOD_QUALITY.len()).map(|_| None).collect(),
            }
          )
        },
      };

    if mip_mesh.lods[lod_index].is_none() {
      *mip_mesh.lods.get_mut(lod_index).unwrap() = Some(
        TerrainBlock::generate(
          timers,
          id_allocator,
          &self.heightmap,
          position,
          LOD_QUALITY[lod_index],
        )
      );
    }

    mip_mesh.lods[lod_index].as_ref().unwrap()
  }
}
