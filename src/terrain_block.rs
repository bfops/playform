use gl::types::*;
use id_allocator::IdAllocator;
use nalgebra::{normalize, cross};
use nalgebra::{Pnt3, Vec3};
use ncollide::bounding_volume::{AABB, AABB3};
use noise::source::Perlin;
use noise::model::Plane;
use state::EntityId;
use std::collections::hash_map::HashMap;
use std::num::Float;
use stopwatch::TimerSet;
use terrain::{AMPLITUDE, TerrainType};

pub const BLOCK_WIDTH: i32 = 4;
// Number of samples in a single dimension per block.
pub const SAMPLES_PER_BLOCK: uint = 16;
pub const SAMPLE_WIDTH: f32 = BLOCK_WIDTH as f32 / SAMPLES_PER_BLOCK as f32;

#[deriving(Show, PartialEq, Eq, Hash, Copy, Clone)]
pub struct BlockPosition(Pnt3<i32>);

impl BlockPosition {
  #[inline(always)]
  pub fn new(x: i32, y: i32, z: i32) -> BlockPosition {
    BlockPosition(Pnt3::new(x, y, z))
  }

  #[inline(always)]
  pub fn as_pnt<'a>(&'a self) -> &'a Pnt3<i32> {
    let BlockPosition(ref pnt) = *self;
    pnt
  }

  #[inline(always)]
  pub fn as_mut_pnt3<'a>(&'a mut self) -> &'a mut Pnt3<i32> {
    let BlockPosition(ref mut pnt) = *self;
    pnt
  }

  pub fn from_world_position(world_position: &Pnt3<f32>) -> BlockPosition {
    macro_rules! convert_coordinate(
      ($x: expr) => ({
        let x = $x.floor() as i32;
        let x =
          if x < 0 {
            x - (BLOCK_WIDTH - 1)
          } else {
            x
          };
        x / BLOCK_WIDTH
      })
    );
    BlockPosition(
      Pnt3::new(
        convert_coordinate!(world_position.x),
        convert_coordinate!(world_position.y),
        convert_coordinate!(world_position.z),
      )
    )
  }

  pub fn to_world_position(&self) -> Pnt3<f32> {
    Pnt3::new(
      (self.as_pnt().x * BLOCK_WIDTH) as f32,
      (self.as_pnt().y * BLOCK_WIDTH) as f32,
      (self.as_pnt().z * BLOCK_WIDTH) as f32,
    )
  }
}

impl Add<Vec3<i32>, BlockPosition> for BlockPosition {
  fn add(mut self, rhs: Vec3<i32>) -> Self {
    self.as_mut_pnt3().x += rhs.x;
    self.as_mut_pnt3().y += rhs.y;
    self.as_mut_pnt3().z += rhs.z;
    self
  }
}

/// Ductyped heightmap methods.
trait HeightMapExt {
  /// The coordinate of the tile at a given x/z.
  fn point_at(&self, x: f32, z: f32) -> Pnt3<f32>;
  /// The lighting normal of the tile at a given x/z.
  fn normal_at(&self, x: f32, z: f32) -> Vec3<f32>;
}

impl<'a> HeightMapExt for Plane<'a, Perlin> {
  fn point_at(&self, x: f32, z: f32) -> Pnt3<f32> {
    let y = AMPLITUDE * (self.get(x, z) + 1.0) / 2.0;
    Pnt3::new(x, y, z)
  }

  fn normal_at(&self, x: f32, z: f32) -> Vec3<f32> {
    const DELTA: f32 = 0.2;
    let dx = self.point_at(x + DELTA, z).to_vec() - self.point_at(x - DELTA, z).to_vec();
    let dz = self.point_at(x, z + DELTA).to_vec() - self.point_at(x, z - DELTA).to_vec();
    normalize(&cross(&dx, &dz))
  }
}

pub struct TerrainBlock {
  // These Vecs must all be ordered the same way.

  // vertex coordinates flattened into separate GLfloats (x, y, z order)
  pub vertex_coordinates: Vec<GLfloat>,
  // per-vertex normal vectors flattened into separate GLfloats (x, y, z order)
  pub normals: Vec<GLfloat>,
  // per-triangle terrain types
  pub typs: Vec<GLuint>,

  // per-triangle entity IDs
  pub ids: Vec<EntityId>,
  // per-triangle bounding boxes
  pub bounds: HashMap<EntityId, AABB3<GLfloat>>,
}

impl TerrainBlock {
  pub fn empty() -> TerrainBlock {
    TerrainBlock {
      vertex_coordinates: Vec::new(),
      normals: Vec::new(),
      typs: Vec::new(),
      ids: Vec::new(),
      bounds: HashMap::new(),
    }
  }

  pub fn generate(
    timers: &TimerSet,
    id_allocator: &mut IdAllocator<EntityId>,
    heightmap: &Perlin,
    position: &BlockPosition,
  ) -> TerrainBlock {
    timers.time("update.generate_block", || {
      let mut block = TerrainBlock::empty();
      let heightmap = Plane::new(heightmap);

      let x = (position.as_pnt().x * BLOCK_WIDTH) as f32;
      let y = (position.as_pnt().y * BLOCK_WIDTH) as f32;
      let z = (position.as_pnt().z * BLOCK_WIDTH) as f32;

      for dx in range(0, SAMPLES_PER_BLOCK) {
        let x = x + dx as f32 * SAMPLE_WIDTH;
        for dz in range(0, SAMPLES_PER_BLOCK) {
          let z = z + dz as f32 * SAMPLE_WIDTH;
          let position = Pnt3::new(x, y, z);
          TerrainBlock::add_tile(
            timers,
            &heightmap,
            id_allocator,
            &mut block,
            &position,
          );
        }
      }

      block
    })
  }

  fn add_tile<'a>(
    timers: &TimerSet,
    hm: &Plane<'a, Perlin>,
    id_allocator: &mut IdAllocator<EntityId>,
    block: &mut TerrainBlock,
    position: &Pnt3<f32>,
  ) {
    let half_width = SAMPLE_WIDTH / 2.0;
    let center = hm.point_at(position.x + half_width, position.z + half_width);

    if position.y < center.y && center.y <= position.y + BLOCK_WIDTH as f32 {
      timers.time("update.generate_block.add_tile", || {
        let center_normal = hm.normal_at(position.x + half_width, position.z + half_width);

        let x2 = position.x + SAMPLE_WIDTH;
        let z2 = position.z + SAMPLE_WIDTH;

        let ps: [Pnt3<f32>, ..4] =
          [ hm.point_at(position.x, position.z)
          , hm.point_at(position.x, z2)
          , hm.point_at(x2, z2)
          , hm.point_at(x2, position.z)
          ];

        let ns: [Vec3<f32>, ..4] =
          [ hm.normal_at(position.x, position.z)
          , hm.normal_at(position.x, x2)
          , hm.normal_at(x2, z2)
          , hm.normal_at(x2, position.z)
          ];

        let center_lower_than = ps.iter().filter(|v| center.y < v.y).count();

        let terrain_type =
          if center_lower_than >= 3 {
            TerrainType::Dirt
          } else {
            TerrainType::Grass
          };

        macro_rules! place_terrain(
          ($v1: expr,
           $v2: expr,
           $n1: expr,
           $n2: expr,
           $minx: expr,
           $minz: expr,
           $maxx: expr,
           $maxz: expr
          ) => ({
            let mut maxy = $v1.y;
            if $v2.y > $v1.y {
              maxy = $v2.y;
            }
            if center.y > maxy {
              maxy = center.y;
            }

            let id = id_allocator.allocate();
            block.vertex_coordinates.push_all(&[
              $v1.x, $v1.y, $v1.z,
              $v2.x, $v2.y, $v2.z,
              center.x, center.y, center.z,
            ]);
            block.normals.push_all(&[
              $n1.x, $n1.y, $n1.z,
              $n2.x, $n2.y, $n2.z,
              center_normal.x, center_normal.y, center_normal.z,
            ]);
            block.typs.push(terrain_type as GLuint);
            block.ids.push(id);
            block.bounds.insert(
              id,
              AABB::new(
                Pnt3::new($minx, $v1.y, $minz),
                Pnt3::new($maxx, maxy, $maxz),
              ),
            );
          });
        );

        let centr = center; // makes alignment nice
        place_terrain!(&ps[0], &ps[1], &ns[0], &ns[1], ps[0].x, ps[0].z, centr.x, ps[1].z);
        place_terrain!(&ps[1], &ps[2], &ns[1], &ns[2], ps[1].x, centr.z, ps[2].x, ps[2].z);
        place_terrain!(&ps[2], &ps[3], &ns[2], &ns[3], centr.x, centr.z, ps[2].x, ps[2].z);
        place_terrain!(&ps[3], &ps[0], &ns[3], &ns[0], ps[0].x, ps[0].z, ps[3].x, centr.z);
      })
    }
  }
}
