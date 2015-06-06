//! Data structure for a small block of terrain.

use cgmath::{Point3, Vector3, Aabb3};

use entity::EntityId;
use serialize::{Flatten, MemStream, EOF};

// TODO: Move the server-only parts to the server, like BLOCK_WIDTH and sample_info.

pub const LG_WIDTH: i16 = 3;
pub const WIDTH: i32 = 1 << LG_WIDTH;
pub const TEXTURE_WIDTH: [u32; 4] = [32, 16, 8, 2];
pub const TEXTURE_LEN: [usize; 4] = [
  TEXTURE_WIDTH[0] as usize * TEXTURE_WIDTH[0] as usize,
  TEXTURE_WIDTH[1] as usize * TEXTURE_WIDTH[1] as usize,
  TEXTURE_WIDTH[2] as usize * TEXTURE_WIDTH[2] as usize,
  TEXTURE_WIDTH[3] as usize * TEXTURE_WIDTH[3] as usize,
];

pub const LG_EDGE_SAMPLES: [u16; 4] = [3, 2, 1, 0];
/// Quality across different LODs.
/// Quality is the number of times the noise function is sampled along each axis.
pub const EDGE_SAMPLES: [u16; 4] = [
  1 << LG_EDGE_SAMPLES[0],
  1 << LG_EDGE_SAMPLES[1],
  1 << LG_EDGE_SAMPLES[2],
  1 << LG_EDGE_SAMPLES[3],
];

pub const LG_SAMPLE_SIZE: [i16; 4] = [
  LG_WIDTH - LG_EDGE_SAMPLES[0] as i16,
  LG_WIDTH - LG_EDGE_SAMPLES[1] as i16,
  LG_WIDTH - LG_EDGE_SAMPLES[2] as i16,
  LG_WIDTH - LG_EDGE_SAMPLES[3] as i16,
];

#[derive(Debug, Copy, Clone)]
/// [T; 3], but serializable.
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
