//! Terrain allocated in vram-sized chunks.

use gl::types::*;
use cgmath::{Point3, Vector3};
use std;
use terrain_mesh;
use common::id_allocator;

use super::terrain_buffers::VRAM_CHUNK_LENGTH;
use super::entity;

#[allow(missing_docs)]
pub struct T {
  // Every vector should be the same length

  /// Position of each vertex.
  pub vertex_coordinates: Vec<[terrain_mesh::Triangle<Point3<f32>>; VRAM_CHUNK_LENGTH]>,
  /// Vertex normals. These should be normalized!
  pub normals: Vec<[terrain_mesh::Triangle<Vector3<f32>>; VRAM_CHUNK_LENGTH]>,
  /// Material IDs for each triangle.
  pub materials: Vec<[i32; VRAM_CHUNK_LENGTH]>,
  /// per-chunk ids
  pub ids: Vec<entity::id::Terrain>
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
) -> T {
  assert_eq!(vertices.len(), normals.len());
  assert_eq!(vertices.len(), materials.len());
  let len = round_up(vertices.len(), VRAM_CHUNK_LENGTH);
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
  T {
    vertex_coordinates : unsafe { Vec::from_raw_parts(vertices_ptr  as *mut _, vec_len, vec_len) },
    normals            : unsafe { Vec::from_raw_parts(normals_ptr   as *mut _, vec_len, vec_len) },
    materials          : unsafe { Vec::from_raw_parts(materials_ptr as *mut _, vec_len, vec_len) },
    ids                : (0..vec_len).map(|_| id_allocator.allocate()).collect(),
  }
}
