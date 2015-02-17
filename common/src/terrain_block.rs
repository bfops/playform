//! Data structure for a small block of terrain.

use color::Color3;
use entity::EntityId;
use nalgebra::{Pnt2, Pnt3, Vec3};
use ncollide_entities::bounding_volume::AABB3;

pub const BLOCK_WIDTH: i32 = 8;
pub const TEXTURE_WIDTH: [u32; 4] = [32, 16, 8, 2];
pub const TEXTURE_LEN: [usize; 4] = [
  TEXTURE_WIDTH[0] as usize * TEXTURE_WIDTH[0] as usize,
  TEXTURE_WIDTH[1] as usize * TEXTURE_WIDTH[1] as usize,
  TEXTURE_WIDTH[2] as usize * TEXTURE_WIDTH[2] as usize,
  TEXTURE_WIDTH[3] as usize * TEXTURE_WIDTH[3] as usize,
];

/// Quality across different LODs.
/// Quality is the number of times the noise function is sampled along each axis.
pub const LOD_QUALITY: [u16; 4] = [8, 4, 2, 1];

#[derive(Debug, Clone)]
#[derive(RustcDecodable, RustcEncodable)]
/// [T; 3], but deriving RustcDecodable.
pub struct Triangle<T> {
  #[allow(missing_docs)]
  pub v1: T,
  #[allow(missing_docs)]
  pub v2: T,
  #[allow(missing_docs)]
  pub v3: T,
}

/// Construct a triangle.
pub fn tri<T>(v1: T, v2: T, v3: T) -> Triangle<T> {
  Triangle {
    v1: v1,
    v2: v2,
    v3: v3,
  }
}

#[derive(Debug, Clone)]
#[derive(RustcDecodable, RustcEncodable)]
/// A small continguous chunk of terrain.
pub struct TerrainBlock {
  // These Vecs must all be ordered the same way; each entry is the next triangle.

  /// Position of each vertex.
  pub vertex_coordinates: Vec<Triangle<Pnt3<f32>>>,
  /// Vertex normals. These should be normalized!
  pub normals: Vec<Triangle<Vec3<f32>>>,
  /// Per-vertex indices into an array in `pixels`.
  pub coords: Vec<Triangle<Pnt2<f32>>>,
  /// Entity IDs for each triangle.
  pub ids: Vec<EntityId>,
  // TODO: Change this back to a HashMap once initial capacity is zero for those.
  /// Per-triangle bounding boxes.
  pub bounds: Vec<(EntityId, AABB3<f32>)>,

  /// Textures for this block.
  pub pixels: Vec<Color3<f32>>,
}

impl TerrainBlock {
  /// Construct an empty `TerrainBlock`.
  pub fn empty() -> TerrainBlock {
    TerrainBlock {
      vertex_coordinates: Vec::new(),
      normals: Vec::new(),
      coords: Vec::new(),

      pixels: Vec::new(),

      ids: Vec::new(),
      bounds: Vec::new(),
    }
  }
}
