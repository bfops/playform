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
          TerrainBlock::add_tile(
            timers,
            &heightmap,
            id_allocator,
            &mut block,
            &mut face_normals[dx][dz],
            &position,
          );
        }
      }

      // This is all super mesh-specific.

      // Get the sum of the normal vectors for a given index on a given tile.
      macro_rules! tile_normal_sum(
        ($x_tile: expr, $z_tile: expr, $mesh_vertex: expr) => ({
          let mesh_vertex: uint = $mesh_vertex;
          face_normals[$x_tile][$z_tile].map(
            |square_normals|
              match mesh_vertex {
                1 => (square_normals[3] + square_normals[0]),
                2 => (square_normals[0] + square_normals[1]),
                3 => (square_normals[1] + square_normals[2]),
                4 => (square_normals[2] + square_normals[3]),
                _ => unimplemented!(),
              }
            )
        })
      );

      macro_rules! vertex_normal_average(
        ($x_tile: expr, $z_tile: expr) => ({
          let mut sum = Vec3::new(0.0, 0.0, 0.0);
          let mut n = 0.0;

          if $x_tile < SAMPLES_PER_BLOCK && $z_tile < SAMPLES_PER_BLOCK {
            tile_normal_sum!($x_tile, $z_tile, 1).map(
              |s| {
                sum = sum + s;
                n += 2.0;
              });
          }
          if $x_tile > 0 && $z_tile < SAMPLES_PER_BLOCK {
            tile_normal_sum!($x_tile - 1, $z_tile, 4).map(
              |s| {
                sum = sum + s;
                n += 2.0;
              });
          }
          if $x_tile < SAMPLES_PER_BLOCK && $z_tile > 0 {
            tile_normal_sum!($x_tile, $z_tile - 1, 2).map(
              |s| {
                sum = sum + s;
                n += 2.0;
              });
          }
          if $x_tile > 0 && $z_tile > 0 {
            tile_normal_sum!($x_tile - 1, $z_tile - 1, 3).map(
              |s| {
                sum = sum + s;
                n += 2.0;
              });
          }

          sum / n
        })
      );

      const MESH_WIDTH: uint = SAMPLES_PER_BLOCK + 1;

      let mut mesh_vertex_normals =
        [Vec3::new(0.0, 0.0, 0.0), ..MESH_WIDTH * MESH_WIDTH];

      let mut mesh_index = 0;
      for z_tile in range(0, MESH_WIDTH) {
        for x_tile in range(0, MESH_WIDTH) {
          mesh_vertex_normals[mesh_index] = vertex_normal_average!(x_tile, z_tile);
          mesh_index += 1;
        }
      }

      for x_tile in range(0, SAMPLES_PER_BLOCK) {
        for z_tile in range(0, SAMPLES_PER_BLOCK) {
          let mut tile_mesh = [Vec3::new(0.0, 0.0, 0.0), ..5];

          const N_INDICES: uint = 12;

          match face_normals[x_tile][z_tile] {
            None => {
              // This tile isn't part of this block's mesh.
              continue;
            },
            Some(square_normals) => {
              for n in square_normals.iter() {
                tile_mesh[0] = tile_mesh[0] + *n;
              }
              tile_mesh[0] = tile_mesh[0] / 4.0;

              assert_eq!(N_INDICES, VERTICES_PER_TRIANGLE * square_normals.len());
            },
          }

          let mesh_index = z_tile * MESH_WIDTH + x_tile;

          tile_mesh[1] = mesh_vertex_normals[mesh_index];
          tile_mesh[2] = mesh_vertex_normals[mesh_index + MESH_WIDTH];
          tile_mesh[3] = mesh_vertex_normals[mesh_index + MESH_WIDTH + 1];
          tile_mesh[4] = mesh_vertex_normals[mesh_index + 1];

          let triangle_indices: [uint, ..N_INDICES] = [1,2,0,2,3,0,3,4,0,4,1,0];
          for &idx in triangle_indices.iter() {
            block.normals.push(tile_mesh[idx].x);
            block.normals.push(tile_mesh[idx].y);
            block.normals.push(tile_mesh[idx].z);
          }
        }
      }

      block
    })
  }

  #[inline]
  fn add_tile<'a>(
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
      timers.time("update.generate_block.add_tile", || {
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

            {
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

