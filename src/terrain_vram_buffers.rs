use color::Color3;
use gl;
use gl::types::*;
use id_allocator::IdAllocator;
use nalgebra::{Pnt2, Pnt3, Vec3};
use shaders::terrain::TerrainShader;
use state::EntityId;
use std::collections::HashMap;
use yaglw::gl_context::{GLContext,GLContextExistence};
use yaglw::texture::BufferTexture;
use yaglw::texture::TextureUnit;

#[cfg(test)]
use std::mem;

// VRAM bytes
pub const BYTE_BUDGET: usize = 64_000_000;
pub const POLYGON_COST: usize = 80;
// This assumes there exists only one set of TerrainVRAMBuffers.
pub const POLYGON_BUDGET: usize = BYTE_BUDGET / POLYGON_COST;

pub type Triangle<T> = [T; 3];

pub struct TerrainVRAMBuffers<'a> {
  id_to_index: HashMap<EntityId, usize>,
  index_to_id: Vec<EntityId>,

  // TODO: Use yaglw's ArrayHandle.
  empty_array: GLuint,
  length: usize,

  vertex_positions: BufferTexture<'a, Triangle<Pnt3<GLfloat>>>,
  normals: BufferTexture<'a, Triangle<Vec3<GLfloat>>>,
  // 2D coordinates into `pixels`
  coords: BufferTexture<'a, Triangle<Pnt2<u32>>>,
  pixels: BufferTexture<'a, Color3<GLfloat>>,
}

#[test]
fn correct_size() {
  assert!(mem::size_of::<Triangle<Pnt3<GLfloat>>>() == 3 * mem::size_of::<Pnt3<GLfloat>>());
  assert!(mem::size_of::<Pnt2<GLfloat>>() == 2 * mem::size_of::<GLfloat>());
  assert!(mem::size_of::<Pnt3<GLfloat>>() == 3 * mem::size_of::<GLfloat>());
  assert!(mem::size_of::<Vec3<GLfloat>>() == 3 * mem::size_of::<GLfloat>());
}

impl<'a> TerrainVRAMBuffers<'a> {
  pub fn new(
    gl: &'a GLContextExistence,
    gl_context: &mut GLContext,
  ) -> TerrainVRAMBuffers<'a> {
    TerrainVRAMBuffers {
      id_to_index: HashMap::new(),
      index_to_id: Vec::new(),
      empty_array: unsafe {
        let mut empty_array = 0;
        gl::GenVertexArrays(1, &mut empty_array);
        empty_array
      },
      length: 0,
      vertex_positions: BufferTexture::new(gl, gl_context, gl::R32F, POLYGON_BUDGET),
      normals: BufferTexture::new(gl, gl_context, gl::R32F, POLYGON_BUDGET),
      coords: BufferTexture::new(gl, gl_context, gl::R32F, POLYGON_BUDGET),
      pixels: BufferTexture::new(gl, gl_context, gl::R32F, 5),
    }
  }

  pub fn bind_glsl_uniforms(
    &self,
    gl: &mut GLContext,
    texture_unit_alloc: &mut IdAllocator<TextureUnit>,
    shader: &mut TerrainShader,
  ) {
    shader.shader.use_shader(gl);
    let mut bind = |&mut: name, id| {
      let unit = texture_unit_alloc.allocate();
      unsafe {
        gl::ActiveTexture(unit.gl_id());
        gl::BindTexture(gl::TEXTURE_BUFFER, id);
      }
      let loc = shader.shader.get_uniform_location(name);
      unsafe {
        gl::Uniform1i(loc, unit.glsl_id as GLint);
      }
    };

    bind("positions", self.vertex_positions.handle.gl_id);
    bind("normals", self.normals.handle.gl_id);
    bind("coords", self.coords.handle.gl_id);
    bind("pixels", self.pixels.handle.gl_id);
  }

  pub fn push(
    &mut self,
    gl: &mut GLContext,
    vertices: &[Triangle<Pnt3<GLfloat>>],
    normals: &[Triangle<Vec3<GLfloat>>],
    coords: &[Triangle<Pnt2<u32>>],
    ids: &[EntityId],
  ) -> bool {
    assert_eq!(vertices.len(), ids.len());
    assert_eq!(normals.len(), ids.len());
    assert_eq!(coords.len(), ids.len());

    self.vertex_positions.buffer.byte_buffer.bind(gl);
    let r = self.vertex_positions.buffer.push(gl, vertices);
    if !r {
      return false;
    }

    self.normals.buffer.byte_buffer.bind(gl);
    let r = self.normals.buffer.push(gl, normals);
    assert!(r);

    self.coords.buffer.byte_buffer.bind(gl);
    let r = self.coords.buffer.push(gl, coords);
    assert!(r);

    for &id in ids.iter() {
      self.id_to_index.insert(id, self.index_to_id.len());
      self.index_to_id.push(id);
    }

    self.length += 3 * ids.len();

    true
  }

  pub fn push_pixels(
    &mut self,
    gl: &mut GLContext,
    pixels: &[Color3<GLfloat>],
  ) {
    self.pixels.buffer.byte_buffer.bind(gl);
    let success = self.pixels.buffer.push(gl, pixels);
    assert!(success);
  }

  // Note: `id` must be present in the buffers.
  pub fn swap_remove(&mut self, gl: &mut GLContext, id: EntityId) {
    let idx = *self.id_to_index.get(&id).unwrap();
    let swapped_id = self.index_to_id[self.index_to_id.len() - 1];
    self.index_to_id.swap_remove(idx);
    self.id_to_index.remove(&id);

    if id != swapped_id {
      self.id_to_index.insert(swapped_id, idx);
    }

    self.length -= 3;
    self.vertex_positions.buffer.byte_buffer.bind(gl);
    self.vertex_positions.buffer.swap_remove(gl, idx, 1);
    self.normals.buffer.byte_buffer.bind(gl);
    self.normals.buffer.swap_remove(gl, idx, 1);
    self.coords.buffer.byte_buffer.bind(gl);
    self.coords.buffer.swap_remove(gl, idx, 1);
  }

  pub fn draw(&self, _gl: &mut GLContext) {
    unsafe {
      gl::BindVertexArray(self.empty_array);
      gl::DrawArrays(gl::TRIANGLES, 0, self.length as GLint);
    }
  }
}
