use common::*;
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
      });
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
      let mut face_normals = [[None, ..SAMPLES_PER_BLOCK], ..SAMPLES_PER_BLOCK];

      let heightmap = Plane::new(heightmap);
      let x = (position.as_pnt().x * BLOCK_WIDTH) as f32;
      let y = (position.as_pnt().y * BLOCK_WIDTH) as f32;
      let z = (position.as_pnt().z * BLOCK_WIDTH) as f32;
      for dx in range(0, SAMPLES_PER_BLOCK) {
        let x = x + dx as f32 * SAMPLE_WIDTH;
        for dz in range(0, SAMPLES_PER_BLOCK) {
          let z = z + dz as f32 * SAMPLE_WIDTH;
          let position = Pnt3::new(x, y, z);
          TerrainBlock::add_square(
            timers,
            &heightmap,
            id_allocator,
            &mut block,
            &mut face_normals[dx][dz],
            &position,
          );
        }
      }

      if USE_LIGHTING {
        // This is all super mesh-specific.
        for x_square in range(0, SAMPLES_PER_BLOCK) {
          for z_square in range(0, SAMPLES_PER_BLOCK) {
            match face_normals[x_square][z_square] {
              None => {},
              Some(square_normals) => {
                let triangle_faces = [0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3];

                for &triangle_face in triangle_faces.iter() {
                  block.normals.push_all(&[
                    square_normals[triangle_face].x,
                    square_normals[triangle_face].y,
                    square_normals[triangle_face].z,
                  ]);
                }
              },
            }
          }
        }
      }

      block
    })
  }

  #[inline]
  fn add_square<'a>(
    timers: &TimerSet,
    heightmap: &Plane<'a, Perlin>,
    id_allocator: &mut IdAllocator<EntityId>,
    block: &mut TerrainBlock,
    face_normals: &mut Option<[Vec3<f32>, ..4]>,
    position: &Pnt3<f32>,
  ) {
    macro_rules! at(
      ($x: expr, $z: expr) => ({
        let y = AMPLITUDE * (heightmap.get::<GLfloat>($x, $z) + 1.0) / 2.0;
        Pnt3::new($x, y, $z)
      });
    );

    let half_width = SAMPLE_WIDTH / 2.0;
    let center = at!(position.x + half_width, position.z + half_width);

    if position.y < center.y && center.y <= position.y + BLOCK_WIDTH as f32 {
      timers.time("update.generate_block.add_square", || {
        let mut new_face_normals = [Vec3::new(0.0, 0.0, 0.0), ..4];

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

        macro_rules! place_terrain(
          ($normal_idx: expr, $v1: expr, $v2: expr, $minx: expr, $minz: expr, $maxx: expr, $maxz: expr) => ({
            let mut maxy = $v1.y;
            if $v2.y > $v1.y {
              maxy = $v2.y;
            }
            if center.y > maxy {
              maxy = center.y;
            }

            if USE_LIGHTING {
              let side1: Vec3<GLfloat> = center - *$v1;
              let side2: Vec3<GLfloat> = $v2.to_vec() - $v1.to_vec();
              let normal: Vec3<GLfloat> = normalize(&cross(&side1, &side2));
              new_face_normals[$normal_idx] = normal;
            }

            let id = id_allocator.allocate();
            block.vertex_coordinates.push_all(&[
              $v1.x, $v1.y, $v1.z,
              $v2.x, $v2.y, $v2.z,
              center.x, center.y, center.z,
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

        place_terrain!(0, &v1, &v2, v1.x, v1.z, center.x, v2.z);
        place_terrain!(1, &v2, &v3, v2.x, center.z, v3.x, v3.z);
        place_terrain!(2, &v3, &v4, center.x, center.z, v3.x, v3.z);
        place_terrain!(3, &v4, &v1, v1.x, v1.z, v4.x, center.z);

        *face_normals = Some(new_face_normals);
      })
    }
  }
}

