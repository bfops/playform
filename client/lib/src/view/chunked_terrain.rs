//! Terrain allocated in vram-sized chunks.

use gl::types::*;
use cgmath::{Point3, Vector3};
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

#[allow(missing_docs)]
pub struct PushGrass {
  #[allow(missing_docs)]
  pub tex_id : u32,
  #[allow(missing_docs)]
  pub id     : entity::id::Grass,
}

impl T {
  /// Number of polygons pushed. Note that this may not divide cleanly into a number of chunks.
  pub fn polygon_count(&self) -> usize {
    if self.next_idx_inside_chunks > 0 {
      terrain_buffers::CHUNK_LENGTH * (self.ids.len() - 1) + self.next_idx_inside_chunks
    } else {
      terrain_buffers::CHUNK_LENGTH * self.ids.len()
    }
  }

  /// Number of chunks pushed. Note that this does not naively translate to a polygon count.
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
      let id = id_allocator.allocate();
      self.ids.push(id);
    }

    let chunk_id = *self.ids.last().unwrap();

    self.vertex_coordinates.last_mut().unwrap().0[self.next_idx_inside_chunks] = vertices;
    self.normals.last_mut().unwrap().0[self.next_idx_inside_chunks] = normals;
    self.materials.last_mut().unwrap().0[self.next_idx_inside_chunks] = material;

    grass.map(|grass| {
      self.grass.polygon_chunk_ids.push(chunk_id);
      self.grass.polygon_offsets.push(index::of_u32(self.next_idx_inside_chunks as u32));
      self.grass.tex_ids.push(grass.tex_id);
      self.grass.ids.push(grass.id);
    });

    self.next_idx_inside_chunks = (self.next_idx_inside_chunks + 1) % terrain_buffers::CHUNK_LENGTH;
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
