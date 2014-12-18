use common::*;
use gl::types::*;
use id_allocator::IdAllocator;
use nalgebra::{normalize, cross};
use nalgebra::{Pnt3, Vec3};
use ncollide::bounding_volume::{AABB, AABB3};
use noise::source::Perlin;
use noise::model::Plane;
use state::EntityId;
use std::collections::hash_map::{HashMap, Occupied, Vacant};
use std::mem;
use std::num::Float;
use stopwatch::TimerSet;

pub const BLOCK_WIDTH: int = 4;
// Number of samples in a single dimension per block.
pub const SAMPLES_PER_BLOCK: int = 16;
pub const SAMPLE_WIDTH: f32 = BLOCK_WIDTH as f32 / SAMPLES_PER_BLOCK as f32;

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

pub type BlockPosition = Pnt3<int>;

/// This struct contains and lazily generates the world's terrain.
pub struct Terrain<'a> {
  // this is used for generating new blocks.
  pub heightmap: Perlin,
  // all the blocks that have ever been created.
  pub all_blocks: HashMap<BlockPosition, TerrainBlock>,
}

impl<'a> Terrain<'a> {
  pub fn new() -> Terrain<'a> {
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
  pub fn to_block_position(world_position: Pnt3<f32>) -> BlockPosition {
    macro_rules! convert_coordinate(
      ($x: expr) => ({
        let x = $x.floor() as int;
        let x =
          if x < 0 {
            x - (BLOCK_WIDTH - 1)
          } else {
            x
          };
        x / BLOCK_WIDTH
      });
    )
    let position =
      Pnt3::new(
        convert_coordinate!(world_position.x),
        convert_coordinate!(world_position.y),
        convert_coordinate!(world_position.z),
      );
    position
  }

  // Once a reference is returned from `load`, is should be valid forever.
  // TODO: Figure out a way to specify that the reference can only be valid
  // until this object is touched again (or deleted).
  #[inline]
  pub fn load(
    &'a mut self,
    timers: &TimerSet,
    id_allocator: &mut IdAllocator<EntityId>,
    position: &BlockPosition,
  ) -> &'a TerrainBlock {
    match self.all_blocks.entry(*position) {
      Occupied(entry) => unsafe {
        // Escape lifetime bounds.
        mem::transmute(entry.get())
      },
      Vacant(entry) => {
        let heightmap = &self.heightmap;
        let block =
          timers.time("update.generate_block", || {
            Terrain::generate_block(
              timers,
              id_allocator,
              heightmap,
              position,
            )
          });
        let block: &TerrainBlock = entry.set(block);
        block
      },
    }
  }

  #[inline]
  pub fn generate_block(
    timers: &TimerSet,
    id_allocator: &mut IdAllocator<EntityId>,
    heightmap: &Perlin,
    position: &BlockPosition,
  ) -> TerrainBlock {
    let mut block = TerrainBlock::new();

    let heightmap = Plane::new(heightmap);
    let x = (position.x * BLOCK_WIDTH) as f32;
    let y = (position.y * BLOCK_WIDTH) as f32;
    let z = (position.z * BLOCK_WIDTH) as f32;
    for dx in range(0, SAMPLES_PER_BLOCK) {
      let x = x + dx as f32 * SAMPLE_WIDTH;
      for dz in range(0, SAMPLES_PER_BLOCK) {
        let z = z + dz as f32 * SAMPLE_WIDTH;
        let position = Pnt3::new(x, y, z);
        Terrain::add_square(
          timers,
          &heightmap,
          id_allocator,
          &mut block,
          &position
        );
      }
    }

    block
  }

  #[inline]
  fn add_square(
    timers: &TimerSet,
    heightmap: &Plane<'a, Perlin>,
    id_allocator: &mut IdAllocator<EntityId>,
    block: &mut TerrainBlock,
    position: &Pnt3<f32>,
  ) {
    macro_rules! at(
      ($x: expr, $z: expr) => ({
        let y = AMPLITUDE * (heightmap.get::<GLfloat>($x, $z) + 1.0) / 2.0;
        Pnt3::new($x, y, $z)
      });
    )

    let half_width = SAMPLE_WIDTH / 2.0;
    let center = at!(position.x + half_width, position.z + half_width);

    if position.y < center.y && center.y <= position.y + BLOCK_WIDTH as f32 {
      timers.time("update.generate_block.add_square", || {
        let x2 = position.x + SAMPLE_WIDTH;
        let z2 = position.z + SAMPLE_WIDTH;
        let v1 = at!(position.x, position.z);
        let v2 = at!(position.x, z2);
        let v3 = at!(x2, z2);
        let v4 = at!(x2, position.z);
        let mut center_lower_than: int = 0;
        for v in [v1, v2, v3, v4].iter() {
          if center.y < v.y {
            center_lower_than += 1;
          }
        }
        let terrain_type =
          if center_lower_than >= 3 {
            TerrainType::Dirt
          } else {
            TerrainType::Grass
          };

        let place_terrain = |v1: &Pnt3<GLfloat>, v2: &Pnt3<GLfloat>, minx, minz, maxx, maxz| {
          let mut maxy = v1.y;
          if v2.y > v1.y {
            maxy = v2.y;
          }
          if center.y > maxy {
            maxy = center.y;
          }

          if USE_LIGHTING {
            let side1: Vec3<GLfloat> = center - *v1;
            let side2: Vec3<GLfloat> = v2.to_vec() - v1.to_vec();
            let normal: Vec3<GLfloat> = normalize(&cross(&side1, &side2));
            block.normals.push_all(&[
              normal.x, normal.y, normal.z,
            ]);
          }

          let id = id_allocator.allocate();
          block.vertices.push_all(&[
            v1.x, v1.y, v1.z,
            v2.x, v2.y, v2.z,
            center.x, center.y, center.z,
          ]);
          block.typs.push(terrain_type as GLuint);
          block.ids.push(id);
          block.bounds.insert(
            id,
            AABB::new(
              Pnt3::new(minx, v1.y, minz),
              Pnt3::new(maxx, maxy, maxz),
            ),
          );
        };

        place_terrain(&v1, &v2, v1.x, v1.z, center.x, v2.z);
        place_terrain(&v2, &v3, v2.x, center.z, v3.x, v3.z);
        place_terrain(&v3, &v4, center.x, center.z, v3.x, v3.z);
        place_terrain(&v4, &v1, v1.x, v1.z, v4.x, center.z);
      })
    }
  }
}

pub struct TerrainBlock {
  pub vertices: Vec<GLfloat>,
  pub normals: Vec<GLfloat>,
  pub typs: Vec<GLuint>,
  pub ids: Vec<EntityId>,
  pub bounds: HashMap<EntityId, AABB3<GLfloat>>,
}

impl TerrainBlock {
  pub fn new() -> TerrainBlock {
    TerrainBlock {
      vertices: Vec::new(),
      normals: Vec::new(),
      typs: Vec::new(),
      ids: Vec::new(),
      bounds: HashMap::new(),
    }
  }
}
