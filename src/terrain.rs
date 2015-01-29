use color::Color3;
use id_allocator::IdAllocator;
use noise::Seed;
use state::EntityId;
use std::collections::hash_map::{HashMap, Entry};
use std::iter::range_inclusive;
use stopwatch::TimerSet;
use terrain_block::{TerrainBlock, BlockPosition};
use terrain_heightmap::HeightMap;
use tree_placer::TreePlacer;

// Quality is the number of times the noise function is sampled along each axis.
pub const LOD_QUALITY: [u32; 4] = [8, 4, 2, 1];

pub const AMPLITUDE: f64 = 64.0;
pub const FREQUENCY: f64 = 1.0 / 64.0;
pub const PERSISTENCE: f64 = 1.0 / 16.0;
pub const LACUNARITY: f64 = 8.0;
pub const OCTAVES: usize = 2;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TerrainType {
  Grass,
  Dirt,
  Stone,
  Wood,
  Leaf,
}

impl TerrainType {
  pub fn color(&self) -> Color3<f32> {
    match *self {
      TerrainType::Grass => Color3::of_rgb(0.0, 0.5, 0.0),
      TerrainType::Dirt => Color3::of_rgb(0.5, 0.4, 0.2),
      TerrainType::Stone => Color3::of_rgb(0.5, 0.5, 0.5),
      TerrainType::Wood => Color3::of_rgb(0.4, 0.3, 0.1),
      TerrainType::Leaf => Color3::of_rgb(0.0, 0.4, 0.0),
    }
  }
}

pub struct TerrainMipMesh {
  pub lods: Vec<Option<TerrainBlock>>,
}

/// This struct contains and lazily generates the world's terrain.
pub struct Terrain {
  pub heightmap: HeightMap,
  pub treemap: TreePlacer,
  // all the blocks that have ever been created.
  pub all_blocks: HashMap<BlockPosition, TerrainMipMesh>,
}

impl Terrain {
  pub fn new(terrain_seed: Seed, tree_seed: u32) -> Terrain {
    Terrain {
      heightmap:
        HeightMap::new(terrain_seed, OCTAVES, FREQUENCY, PERSISTENCE, LACUNARITY, AMPLITUDE),
      treemap: TreePlacer::new(tree_seed),
      all_blocks: HashMap::new(),
    }
  }

  pub fn load<F, T>(
    &mut self,
    timers: &TimerSet,
    id_allocator: &mut IdAllocator<EntityId>,
    position: &BlockPosition,
    lod_index: u32,
    f: F,
  ) -> T
    where F: FnOnce(&TerrainBlock) -> T
  {
    let heightmap = &self.heightmap;
    let treemap = &self.treemap;

    macro_rules! load_lod(
      ($mip_mesh: expr) => ({
        let lod_index = lod_index as usize;
        for _ in range_inclusive($mip_mesh.lods.len(), lod_index) {
          $mip_mesh.lods.push(None);
        }
        let mesh = $mip_mesh.lods.get_mut(lod_index).unwrap();
        let lod_index = lod_index as u32;
        if mesh.is_none() {
          *mesh = Some(
            TerrainBlock::generate(
              timers,
              id_allocator,
              heightmap,
              treemap,
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
