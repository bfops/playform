use common::*;
use gl;
use gl::types::*;
use id_allocator::IdAllocator;
use state::EntityId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use yaglw::gl_context::{GLContext,GLContextExistence};
use yaglw::shader::Shader;
use yaglw::texture::BufferTexture;
use yaglw::texture::TextureUnit;

// VRAM bytes
pub const BYTE_BUDGET: uint = 76_000_000;
pub const POLYGON_COST: uint = 76;
// This assumes there exists only one set of TerrainVRAMBuffers.
pub const POLYGON_BUDGET: uint = BYTE_BUDGET / POLYGON_COST;

pub struct TerrainVRAMBuffers<'a> {
  id_to_index: HashMap<EntityId, uint>,
  index_to_id: Vec<EntityId>,

  empty_array: GLuint,
  length: uint,

  // Each position is buffered as 3 separate floats due to image format restrictions.
  vertex_positions: BufferTexture<'a, GLfloat>,
  // Each normal component is buffered separately floats due to image format restrictions.
  normals: BufferTexture<'a, GLfloat>,
  types: BufferTexture<'a, GLuint>,
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
      // There are 3 R32F components per vertex.
      vertex_positions: BufferTexture::new(gl, gl_context, gl::R32F, 3 * VERTICES_PER_TRIANGLE * POLYGON_BUDGET),
      // There are 3 R32F components per normal.
      normals: BufferTexture::new(gl, gl_context, gl::R32F, 3 * VERTICES_PER_TRIANGLE * POLYGON_BUDGET),
      types: BufferTexture::new(gl, gl_context, gl::R32UI, POLYGON_BUDGET),
    }
  }

  pub fn bind_glsl_uniforms(
    &self,
    gl: &mut GLContext,
    texture_unit_alloc: &mut IdAllocator<TextureUnit>,
    shader: Rc<RefCell<Shader>>,
  ) {
    shader.borrow().use_shader(gl);
    let mut bind = |&mut: name, id| {
      let unit = texture_unit_alloc.allocate();
      unsafe {
        gl::ActiveTexture(unit.gl_id());
        gl::BindTexture(gl::TEXTURE_BUFFER, id);
      }
      let loc = shader.borrow_mut().get_uniform_location(name);
      unsafe {
        gl::Uniform1i(loc, unit.glsl_id as GLint);
      }
    };

    bind("positions", self.vertex_positions.handle.gl_id);
    bind("normals", self.normals.handle.gl_id);
    bind("terrain_types", self.types.handle.gl_id);
  }

  pub fn push(
    &mut self,
    gl: &mut GLContext,
    vertices: &[GLfloat],
    normals: &[GLfloat],
    types: &[GLuint],
    ids: &[EntityId],
  ) {
    assert_eq!(vertices.len(), 3 * VERTICES_PER_TRIANGLE * ids.len());
    assert_eq!(normals.len(), vertices.len());
    assert_eq!(types.len(), ids.len());

    for &id in ids.iter() {
      self.id_to_index.insert(id, self.index_to_id.len());
      self.index_to_id.push(id);
    }

    self.length += 3 * ids.len();
    self.vertex_positions.buffer.byte_buffer.bind(gl);
    self.vertex_positions.buffer.push(gl, vertices);
    self.normals.buffer.byte_buffer.bind(gl);
    self.normals.buffer.push(gl, normals);
    self.types.buffer.byte_buffer.bind(gl);
    self.types.buffer.push(gl, types);
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
    self.vertex_positions.buffer.swap_remove(gl, idx * 3 * VERTICES_PER_TRIANGLE, 3 * VERTICES_PER_TRIANGLE);
    self.normals.buffer.byte_buffer.bind(gl);
    self.normals.buffer.swap_remove(gl, 3 * idx * VERTICES_PER_TRIANGLE, 3 * VERTICES_PER_TRIANGLE);
    self.types.buffer.byte_buffer.bind(gl);
    self.types.buffer.swap_remove(gl, idx, 1);
  }

  pub fn draw(&self, _gl: &mut GLContext) {
    unsafe {
      gl::BindVertexArray(self.empty_array);
      gl::DrawArrays(gl::TRIANGLES, 0, self.length as GLint);
    }
  }
}
