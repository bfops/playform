//! Data structures for loading/unloading/maintaining terrain data in VRAM.

use gl;
use gl::types::*;
use cgmath::{Point3, Vector3};
use yaglw;
use yaglw::gl_context::GLContext;
use yaglw::texture::BufferTexture;
use yaglw::texture::TextureUnit;

use common::entity_id;
use common::fnv_map;
use common::id_allocator;

use terrain_mesh::Triangle;

#[cfg(test)]
use std::mem;

const VERTICES_PER_TRIANGLE: u32 = 3;

// VRAM bytes
pub const BYTE_BUDGET: usize = 64_000_000;
pub const POLYGON_COST: usize = 100;
pub const POLYGON_BUDGET: usize = BYTE_BUDGET / POLYGON_COST;

// Instead of storing individual vertices, normals, etc. in VRAM, store them in chunks.
// This makes it much faster to unload things.
pub const VRAM_CHUNK_LENGTH: usize = 1024;
pub const CHUNK_BUDGET: usize = POLYGON_BUDGET / VRAM_CHUNK_LENGTH;

/// Struct for loading/unloading/maintaining terrain data in VRAM.
pub struct T<'a> {
  id_to_index: fnv_map::T<entity_id::T, usize>,
  index_to_id: Vec<entity_id::T>,

  // TODO: Use yaglw's ArrayHandle.
  empty_array: GLuint,
  length: usize,

  // Per-triangle buffers

  vertex_positions: BufferTexture<'a, [Triangle<Point3<GLfloat>>; VRAM_CHUNK_LENGTH]>,
  normals: BufferTexture<'a, [Triangle<Vector3<GLfloat>>; VRAM_CHUNK_LENGTH]>,
  materials: BufferTexture<'a, [GLint; VRAM_CHUNK_LENGTH]>,
}

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
    id: entity_id::T,
  ) -> Option<u32> {
    self.id_to_index.get(&id).map(|&x| x as u32)
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

  pub fn bind_vertex_positions(
    &self,
    gl: &mut GLContext,
    texture_unit_alloc: &mut id_allocator::T<TextureUnit>,
    shader: &mut yaglw::shader::Shader,
  ) {
    shader.use_shader(gl);
    self.bind(texture_unit_alloc, shader, "positions", self.vertex_positions.handle.gl_id);
  }

  pub fn bind_normals(
    &self,
    gl: &mut GLContext,
    texture_unit_alloc: &mut id_allocator::T<TextureUnit>,
    shader: &mut yaglw::shader::Shader,
  ) {
    shader.use_shader(gl);
    self.bind(texture_unit_alloc, shader, "normals", self.normals.handle.gl_id);
  }

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
    gl: &mut GLContext,
    vertices: Vec<Triangle<Point3<GLfloat>>>,
    normals: Vec<Triangle<Vector3<GLfloat>>>,
    ids: Vec<entity_id::T>,
    materials: Vec<GLint>,
  ) {
    assert_eq!(vertices.len(), ids.len());
    assert_eq!(normals.len(), ids.len());
    assert_eq!(materials.len(), ids.len());

    let diff = VRAM_CHUNK_LENGTH as isize - ids.len() as isize;
    if diff < 0 {
      warn!("Skipping chunk of size {}", ids.len());
    } else if diff > 0 {
      let diff = diff as usize;
      let point = Point3::new(0.0, 0.0, 0.0);
      let normal = Vector3::new(0.0, 0.0, 0.0);
      vertices.extend(&[Triangle { v1: point, v2: point, v3: point }; diff]);
      normals.extend(&[Triangle { v1: normal, v2: normal, v3: normal }; diff]);
      materials.extend(&[0; diff]);
    }

    self.vertex_positions.buffer.byte_buffer.bind(gl);
    let success = self.vertex_positions.buffer.push(gl, vertices);
    assert!(success);

    self.normals.buffer.byte_buffer.bind(gl);
    let success = self.normals.buffer.push(gl, normals);
    assert!(success);

    for &id in ids.iter() {
      self.id_to_index.insert(id, self.index_to_id.len());
      self.index_to_id.push(id);
    }

    self.materials.buffer.byte_buffer.bind(gl);
    let success = self.materials.buffer.push(gl, materials);
    assert!(success);

    self.length += VERTICES_PER_TRIANGLE as usize * ids.len();
  }

  // Note: `id` must be present in the buffers.
  /// Remove some entity from VRAM.
  /// Returns the swapped ID and its VRAM index, if any.
  pub fn swap_remove(
    &mut self,
    gl: &mut GLContext,
    id: entity_id::T,
  ) -> Option<(entity_id::T, usize)>
  {
    let idx = (*self).id_to_index[&id];
    let swapped_id = self.index_to_id[self.index_to_id.len() - 1];
    self.index_to_id.swap_remove(idx);
    self.id_to_index.remove(&id);

    let r =
      if id == swapped_id {
        None
      } else {
        self.id_to_index.insert(swapped_id, idx);
        Some((swapped_id, idx))
      };

    self.length -= 3;

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
      gl::DrawArrays(gl::TRIANGLES, 0, self.length as GLint);
    }
  }
}
