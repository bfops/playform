//! Terrain allocated in vram-sized chunks.

use gl::types::*;
use cgmath::{Point3, Vector3};
use std;
use terrain_mesh;

use common::id_allocator;
use common::index;

use super::terrain_buffers;
use super::terrain_buffers::Chunk;
use super::entity;

/// Information required to load a grass tuft into vram, transformed to refer to vram terrain chunks.
// TODO: Consider making fields non-pub and exposing read-only accessors.
pub struct Grass {
  /// subtexture indices
  pub tex_ids : Vec<u32>,
  #[allow(missing_docs)]
  pub ids : Vec<entity::id::Grass>,
  /// id of the vram terrain chunk for each polygon that the grass tufts rest on
  pub polygon_chunk_ids : Vec<entity::id::Terrain>,
  /// offset, relative to the beginning of the (vram) chunk, of the terrain polygon that a grass tuft rests on
  pub polygon_offsets : Vec<index::T<Chunk<terrain_buffers::Polygon>, terrain_buffers::Polygon>>,
}

impl Grass {
  fn empty() -> Self {
    Grass {
      tex_ids           : Vec::new(),
      ids               : Vec::new(),
      polygon_chunk_ids : Vec::new(),
      polygon_offsets   : Vec::new(),
    }
  }

  #[allow(missing_docs)]
  pub fn len(&self) -> usize {
    self.ids.len()
  }
}

#[allow(missing_docs)]
// TODO: Consider making fields non-pub and exposing read-only accessors.
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

  /// The index within each `Chunk` that we should write to next when pushing new data.
  next_idx_inside_chunks: usize,
}

pub struct PushGrass {
  tex_id : u32,
  id     : entity::id::Grass,
}

impl T {
  pub fn polygon_count(&self) -> usize {
    if self.next_idx_inside_chunks > 0 {
      terrain_buffers::CHUNK_LENGTH * (self.ids.len() - 1) + self.next_idx_inside_chunks
    } else {
      terrain_buffers::CHUNK_LENGTH * self.ids.len()
    }
  }

  pub fn chunk_count(&self) -> usize {
    self.ids.len()
  }

  /// is there nothing to be loaded in this chunk?
  pub fn is_empty(&self) -> bool {
    self.chunk_count() == 0
  }

  #[allow(missing_docs)]
  pub fn push(
    &mut self,
    id_allocator : &mut id_allocator::T<entity::id::Terrain>,
    vertices     : terrain_mesh::Triangle<Point3<GLfloat>>,
    normals      : terrain_mesh::Triangle<Vector3<GLfloat>>,
    material     : GLint,
    grass        : Option<PushGrass>,
  ) {
    // After this block executes, then it is unconditionally true that we write to the last chunk in every `Vec` at this index.
    // We only allocate a new chunk when we know we will actually write data to it, to avoid conceptual ambiguity between the
    // lack of a pushed chunk vs an empty pushed chunk (e.g. for the return value of `len`)
    if self.next_idx_inside_chunks == 0 {
      let zero = Point3::new(0.0, 0.0, 0.0);
      self.vertex_coordinates.push(terrain_buffers::Chunk([terrain_mesh::tri(zero, zero, zero); terrain_buffers::CHUNK_LENGTH]));
      let zero = Vector3::new(0.0, 0.0, 0.0);
      self.normals.push(terrain_buffers::Chunk([terrain_mesh::tri(zero, zero, zero); terrain_buffers::CHUNK_LENGTH]));
      self.materials.push(terrain_buffers::Chunk([0; terrain_buffers::CHUNK_LENGTH]));
    }

    self.vertex_coordinates.last_mut().unwrap().0[self.next_idx_inside_chunks] = vertices;
    self.normals.last_mut().unwrap().0[self.next_idx_inside_chunks] = normals;
    self.materials.last_mut().unwrap().0[self.next_idx_inside_chunks] = material;

    let id = id_allocator.allocate();
    grass.map(|grass| {
      self.grass.polygon_chunk_ids.push(id);
      self.grass.polygon_offsets.push(index::of_u32(self.next_idx_inside_chunks));
      self.grass.tex_ids.push(grass.tex_id);
      self.grass.ids.push(grass.id);
    });

    self.next_idx_inside_chunks += 1;
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
pub fn empty() -> T {
  T {
    vertex_coordinates     : Vec::new(),
    normals                : Vec::new(),
    materials              : Vec::new(),
    ids                    : Vec::new(),
    grass                  : Grass::empty(),
    next_idx_inside_chunks : 0
  }
}
