//! Terrain allocated in vram-sized chunks.

use gl::types::*;
use cgmath::{Point3, Vector3};
use std;
use terrain_mesh;
use common::id_allocator;

use super::terrain_buffers::VRAM_CHUNK_LENGTH;
use super::entity;

pub struct Grass {
  /// subtexture indices
  pub tex_ids : Vec<u32>,
  #[allow(missing_docs)]
  pub ids : Vec<entity::id::Grass>,
  /// offset, relative to the beginning of the chunk, of the terrain polygon that a grass tuft rests on
  pub chunk_ids : Vec<entity::id::Terrain>,
  pub polygon_offsets : Vec<index::T<Chunk<terrain_buffers::Polygon>, terrain_buffers::Polygon>>,
}

impl Grass {
  #[allow(missing_docs)]
  pub fn len(&self) -> usize {
    self.ids.len()
  }
}

#[allow(missing_docs)]
pub struct T {
  // Every vector should be the same length

  /// Position of each vertex.
  pub vertex_coordinates: Vec<Chunk<terrain_mesh::Triangle<Point3<f32>>>>,
  /// Vertex normals. These should be normalized!
  pub normals: Vec<terrain_buffers::Chunk<terrain_mesh::Triangle<Vector3<f32>>>>,
  /// Material IDs for each triangle.
  pub materials: Vec<terrain_buffers::Chunk<i32>>,
  /// per-chunk ids
  pub ids: Vec<entity::id::Terrain>,
  pub grass : Grass,
}

impl T {
  /// is there nothing to be loaded in this chunk?
  pub fn len(&self) -> usize {
    self.ids.len()
  }

  /// is there nothing to be loaded in this chunk?
  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }
}

fn round_up(x: usize, nearest: usize) -> usize {
  ((x + (nearest - 1)) / nearest) * nearest
}

fn zero_pad_to<T>(v: &mut Vec<T>, len: usize) {
  let orig_len = v.len();
  let diff = len - orig_len;
  v.reserve_exact(diff);
  unsafe {
    v.set_len(len);
    std::ptr::write_bytes(v.as_mut_ptr().offset(orig_len as isize), 0, diff);
  }
}

#[allow(missing_docs)]
pub fn of_parts(
  id_allocator  : &mut id_allocator::T<entity::id::Terrain>,
  mut vertices  : Vec<terrain_mesh::Triangle<Point3<GLfloat>>>,
  mut normals   : Vec<terrain_mesh::Triangle<Vector3<GLfloat>>>,
  mut materials : Vec<GLint>,
  grass         : terrain_mesh::Grass,
) -> T {
  assert_eq!(vertices.len(), normals.len());
  assert_eq!(vertices.len(), materials.len());
  let len = round_up(vertices.len(), terrain_buffers::CHUNK_LENGTH);
  zero_pad_to(&mut vertices, len);
  zero_pad_to(&mut normals, len);
  zero_pad_to(&mut materials, len);
  let vec_len = len / VRAM_CHUNK_LENGTH;
  let vertices_ptr  = vertices.as_mut_ptr();
  let normals_ptr   = normals.as_mut_ptr();
  let materials_ptr = materials.as_mut_ptr();
  std::mem::forget(vertices);
  std::mem::forget(normals);
  std::mem::forget(materials);
  let ids = (0..vec_len).map(|_| id_allocator.allocate()).collect();
  T {
    vertex_coordinates : unsafe { Vec::from_raw_parts(vertices_ptr  as *mut _, vec_len, vec_len) },
    normals            : unsafe { Vec::from_raw_parts(normals_ptr   as *mut _, vec_len, vec_len) },
    materials          : unsafe { Vec::from_raw_parts(materials_ptr as *mut _, vec_len, vec_len) },
    ids                : ids,
    grass              : {
      let chunk_ids = Vec::with_capacity(grass.len());
      let polygon_offsets = Vec::with_capacity(grass.len());
      for i in grass.polygon_offsets {
        chunk_ids.push(ids[i / terrain_buffers::CHUNK_LENGTH]);
        polygon_offsets.push(ids[i % terrain_buffers::CHUNK_LENGTH]);
      }
      Grass {
        tex_ids         : grass.tex_ids,
        ids             : grass.ids,
        polygon_offsets : polygon_offsets,
        chunk_ids       : chunk_ids,
      }
    },
  }
}
