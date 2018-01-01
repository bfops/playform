//! Data structures for loading/unloading/maintaining terrain data in VRAM.

use gl;
use gl::types::*;
use cgmath::{Point3, Vector3};
use std;
use yaglw;
use yaglw::gl_context::GLContext;
use yaglw::texture::BufferTexture;
use yaglw::texture::TextureUnit;

use common::index;
use common::fnv_map;
use common::id_allocator;

use terrain_mesh::Triangle;

use super::entity;

#[cfg(test)]
use std::mem;

const VERTICES_PER_TRIANGLE: usize = 3;

/// Maximum number of bytes to be used in VRAM
pub const BYTE_BUDGET: usize = 64_000_000;
const POLYGON_COST: usize = 100;
/// Maximum number of polygons to be used in VRAM
pub const POLYGON_BUDGET: usize = BYTE_BUDGET / POLYGON_COST;

/// Number of elements in a chunk in vram.
pub const CHUNK_LENGTH: usize = 1 << 5;
/// The number of polygons loaded contiguously into VRAM.
const CHUNK_BUDGET: usize = POLYGON_BUDGET / CHUNK_LENGTH;
/// Instead of storing individual vertices, normals, etc. in VRAM, store them in chunks.
/// This makes it much faster to unload things.
pub struct Chunk<V>(pub [V; CHUNK_LENGTH]);

impl<V> Chunk<V> {
  #[allow(missing_docs)]
  pub fn as_ptr(&self) -> *const V {
    self.0.as_ptr()
  }
}

/// Struct for loading/unloading/maintaining terrain data in VRAM.
pub struct T<'a> {
  id_to_index: fnv_map::T<entity::id::Terrain, usize>,
  index_to_id: Vec<entity::id::Terrain>,

  // TODO: Use yaglw's ArrayHandle.
  empty_array: GLuint,
  length: u32,

  // Per-triangle buffers

  vertex_positions: BufferTexture<'a, Chunk<Triangle<Point3<GLfloat>>>>,
  normals: BufferTexture<'a, Chunk<Triangle<Vector3<GLfloat>>>>,
  materials: BufferTexture<'a, Chunk<GLint>>,
}

/// Phantom type for this buffer.
#[derive(Debug)]
pub struct IndexPhantom;
/// Phantom type for Polygons
#[derive(Debug)]
pub struct Polygon([u8; 1]);
#[allow(missing_docs)]
pub type ChunkIndex = index::T<IndexPhantom, Chunk<Polygon>>;
#[allow(missing_docs)]
pub type PolygonIndex = index::T<IndexPhantom, Polygon>;

#[test]
fn correct_size() {
  use cgmath::Point2;

  assert!(mem::size_of::<Triangle<Point3<GLfloat>>>() == 3 * mem::size_of::<Point3<GLfloat>>());
  assert!(mem::size_of::<Point2<GLfloat>>() == 2 * mem::size_of::<GLfloat>());
  assert!(mem::size_of::<Point3<GLfloat>>() == 3 * mem::size_of::<GLfloat>());
  assert!(mem::size_of::<Vector3<GLfloat>>() == 3 * mem::size_of::<GLfloat>());
}

#[allow(missing_docs)]
pub fn new<'a, 'b>(
  gl: &'b mut GLContext,
) -> T<'a> where
  'a: 'b,
{
  T {
    id_to_index: fnv_map::new(),
    index_to_id: Vec::new(),
    empty_array: unsafe {
      let mut empty_array = 0;
      gl::GenVertexArrays(1, &mut empty_array);
      empty_array
    },
    length: 0,
    vertex_positions: BufferTexture::new(gl, gl::R32F, CHUNK_BUDGET),
    normals: BufferTexture::new(gl, gl::R32F, CHUNK_BUDGET),
    materials: BufferTexture::new(gl, gl::R32UI, CHUNK_BUDGET),
  }
}

impl<'a> T<'a> {
  /// Lookup the OpenGL index for an entity.
  pub fn lookup_opengl_index(
    &self,
    id: entity::id::Terrain,
  ) -> Option<ChunkIndex> {
    self.id_to_index.get(&id).map(|&x| x as u32).map(|i| index::of_u32(i))
  }

  fn bind(
    &self,
    texture_unit_alloc: &mut id_allocator::T<TextureUnit>,
    shader: &mut yaglw::shader::Shader,
    name: &'static str,
    id: u32,
  ) {
    let unit = texture_unit_alloc.allocate();
    unsafe {
      gl::ActiveTexture(unit.gl_id());
      gl::BindTexture(gl::TEXTURE_BUFFER, id);
    }
    let loc = shader.get_uniform_location(name);
    unsafe {
      gl::Uniform1i(loc, unit.glsl_id as GLint);
    }
  }

  #[allow(missing_docs)]
  pub fn bind_vertex_positions(
    &self,
    gl: &mut GLContext,
    texture_unit_alloc: &mut id_allocator::T<TextureUnit>,
    shader: &mut yaglw::shader::Shader,
  ) {
    shader.use_shader(gl);
    self.bind(texture_unit_alloc, shader, "positions", self.vertex_positions.handle.gl_id);
  }

  #[allow(missing_docs)]
  pub fn bind_normals(
    &self,
    gl: &mut GLContext,
    texture_unit_alloc: &mut id_allocator::T<TextureUnit>,
    shader: &mut yaglw::shader::Shader,
  ) {
    shader.use_shader(gl);
    self.bind(texture_unit_alloc, shader, "normals", self.normals.handle.gl_id);
  }

  #[allow(missing_docs)]
  pub fn bind_materials(
    &self,
    gl: &mut GLContext,
    texture_unit_alloc: &mut id_allocator::T<TextureUnit>,
    shader: &mut yaglw::shader::Shader,
  ) {
    shader.use_shader(gl);
    self.bind(texture_unit_alloc, shader, "materials", self.materials.handle.gl_id);
  }

  /// Add a series of entites into VRAM.
  pub fn push(
    &mut self,
    gl        : &mut GLContext,
    chunk_id  : entity::id::Terrain,
    vertices  : &Chunk<Triangle<Point3<GLfloat>>>,
    normals   : &Chunk<Triangle<Vector3<GLfloat>>>,
    materials : &Chunk<GLint>,
  ) {
    debug!("Insert {:?}", chunk_id);

    let vertices  = unsafe { std::slice::from_raw_parts(vertices.as_ptr()  as *const _, 1) };
    let normals   = unsafe { std::slice::from_raw_parts(normals.as_ptr()   as *const _, 1) };
    let materials = unsafe { std::slice::from_raw_parts(materials.as_ptr() as *const _, 1) };

    self.vertex_positions.buffer.byte_buffer.bind(gl);
    let success = self.vertex_positions.buffer.push(gl, vertices);
    assert!(success);

    self.normals.buffer.byte_buffer.bind(gl);
    let success = self.normals.buffer.push(gl, normals);
    assert!(success);

    let previous = self.id_to_index.insert(chunk_id, self.index_to_id.len());
    assert!(previous.is_none());
    self.index_to_id.push(chunk_id);
    assert_eq!(self.id_to_index.len(), self.index_to_id.len());

    self.materials.buffer.byte_buffer.bind(gl);
    let success = self.materials.buffer.push(gl, materials);
    assert!(success);

    self.length += 1;
  }

  /// Remove some entity from VRAM.
  /// Returns the swapped ID and its VRAM index, if any.
  pub fn swap_remove(
    &mut self,
    gl: &mut GLContext,
    id: entity::id::Terrain,
  ) -> Option<(ChunkIndex, ChunkIndex)>
  {
    let idx = self.id_to_index[&id];
    let swapped_idx = self.index_to_id.len() - 1;
    let swapped_id = self.index_to_id[swapped_idx];
    self.index_to_id.swap_remove(idx);
    self.id_to_index.remove(&id);

    debug!("Swap-remove {:?} {:?} with {:?} {:?}", id, idx, swapped_id, swapped_idx);

    let r =
      if id == swapped_id {
        None
      } else {
        self.id_to_index.insert(swapped_id, idx);
        Some((index::of_u32(idx as u32), index::of_u32(swapped_idx as u32)))
      };

    self.length -= 1;

    self.vertex_positions.buffer.byte_buffer.bind(gl);
    self.vertex_positions.buffer.swap_remove(gl, idx, 1);

    self.normals.buffer.byte_buffer.bind(gl);
    self.normals.buffer.swap_remove(gl, idx, 1);

    self.materials.buffer.byte_buffer.bind(gl);
    self.materials.buffer.swap_remove(gl, idx, 1);

    r
  }

  /// Draw the terrain.
  pub fn draw(&self, _gl: &mut GLContext) {
    unsafe {
      gl::BindVertexArray(self.empty_array);
      gl::DrawArrays(gl::TRIANGLES, 0, (self.length * CHUNK_LENGTH as u32 * VERTICES_PER_TRIANGLE as u32) as GLint);
    }
  }
}
