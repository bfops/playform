use id_allocator::IdAllocator;
use noise::source::Perlin;
use state::EntityId;
use std::collections::hash_map::{HashMap, Entry};
use std::mem;
use stopwatch::TimerSet;
use terrain_block::{TerrainBlock, BlockPosition};

pub const AMPLITUDE: f32 = 64.0;
pub const FREQUENCY: f64 = 1.0 / 32.0;
pub const PERSISTENCE: f64 = 1.0 / 8.0;
pub const LACUNARITY: f64 = 8.0;
pub const OCTAVES: uint = 6;

#[deriving(Show, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TerrainType {
  Grass,
  Dirt,
  Stone,
}

/// This struct contains and lazily generates the world's terrain.
pub struct Terrain {
  // this is used for generating new blocks.
  pub heightmap: Perlin,
  // all the blocks that have ever been created.
  pub all_blocks: HashMap<BlockPosition, TerrainBlock>,
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

  /// N.B. The references returned from this function may not be valid once this object is touched again.
  // TODO: Figure out a way to specify that the reference can only be valid
  // until this object is touched again (or deleted).
  #[inline]
  pub unsafe fn load(
    &mut self,
    timers: &TimerSet,
    id_allocator: &mut IdAllocator<EntityId>,
    position: &BlockPosition,
  ) -> &TerrainBlock {
    match self.all_blocks.entry(*position) {
      Entry::Occupied(entry) => {
        // Escape lifetime bounds.
        mem::transmute(entry.get())
      },
      Entry::Vacant(entry) => {
        let heightmap = &self.heightmap;
        let block =
          TerrainBlock::generate(
            timers,
            id_allocator,
            heightmap,
            position,
          );
        let block: &TerrainBlock = entry.set(block);
        block
      },
    }
  }
}
