//! Data structure for a small block of terrain.

use cgmath::{Point3, Vector3, Aabb3};

use entity::EntityId;
use serialize::{Flatten, MemStream, EOF};

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

#[derive(Debug, Copy, Clone)]
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
  pub vertex_coordinates: Vec<Triangle<Point3<f32>>>,
  /// Vertex normals. These should be normalized!
  pub normals: Vec<Triangle<Vector3<f32>>>,
  /// Entity IDs for each triangle.
  pub ids: Vec<EntityId>,
  // TODO: Change this back to a HashMap once initial capacity is zero for those.
  /// Per-triangle bounding boxes.
  pub bounds: Vec<(EntityId, Aabb3<f32>)>,
}

impl TerrainBlock {
  /// Construct an empty `TerrainBlock`.
  pub fn empty() -> TerrainBlock {
    TerrainBlock {
      vertex_coordinates: Vec::new(),
      normals: Vec::new(),

      ids: Vec::new(),
      bounds: Vec::new(),
    }
  }
}

flatten_struct_impl!(TerrainBlock, vertex_coordinates, normals, ids, bounds);
