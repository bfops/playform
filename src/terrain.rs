use common::*;
use gl;
use gl::types::*;
use id_allocator::IdAllocator;
use nalgebra::{Pnt3, Vec3};
use state::EntityId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use yaglw::gl_context::{GLContext, GLContextExistence};
use yaglw::shader::Shader;
use yaglw::texture::BufferTexture;
use yaglw::texture::TextureUnit;
use yaglw::vertex_buffer::ArrayHandle;

#[deriving(Show, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TerrainType {
  Grass,
  Dirt,
  Stone,
}

pub struct TerrainPiece {
  pub vertices: [Pnt3<GLfloat>, ..3],
  pub normal: Vec3<GLfloat>,
  pub typ: GLuint,
  pub id: EntityId,
}

pub struct TerrainBuffers<'a> {
  id_to_index: HashMap<EntityId, uint>,
  index_to_id: Vec<EntityId>,

  empty_array: ArrayHandle<'a>,
  length: uint,
  // Each position is buffered as 3 separate floats due to image format restrictions.
  vertex_positions: BufferTexture<'a, GLfloat>,
  // Each normal component is buffered separately floats due to image format restrictions.
  normals: BufferTexture<'a, GLfloat>,
  types: BufferTexture<'a, GLuint>,
}

impl<'a> TerrainBuffers<'a> {
  pub fn new(
    gl: &'a GLContextExistence,
    gl_context: & mut GLContext,
  ) -> TerrainBuffers<'a> {
    let vertex_positions =
      BufferTexture::new(
        gl,
        gl_context,
        gl::R32F,
        3 * MAX_WORLD_SIZE * VERTICES_PER_TRIANGLE,
      );
    let normals = BufferTexture::new(
        gl,
        gl_context,
        gl::R32F,
        3 * MAX_WORLD_SIZE,
      );
    let types =
      BufferTexture::new(
        gl,
        gl_context,
        gl::R32UI,
        MAX_WORLD_SIZE,
      );
    TerrainBuffers {
      id_to_index: HashMap::new(),
      index_to_id: Vec::new(),
      empty_array: ArrayHandle::new(gl),
      length: 0,
      // multiply by 3 because there are 3 R32F components
      vertex_positions: vertex_positions,
      normals: normals,
      types: types,
    }
  }

  pub fn bind(
    &self,
    gl: &mut GLContext,
    texture_unit_alloc: &mut IdAllocator<TextureUnit>,
    shader: Rc<RefCell<Shader>>,
  ) {
    let bind = |name, id| {
      let unit = texture_unit_alloc.allocate();
      unsafe {
        gl::ActiveTexture(unit.gl_id());
        gl::BindTexture(gl::TEXTURE_BUFFER, id);
      }
      let loc = shader.borrow_mut().get_uniform_location(name);
      shader.borrow_mut().use_shader(gl);
      unsafe {
        gl::Uniform1i(loc, unit.glsl_id as GLint);
      }
    };

    bind("positions", self.vertex_positions.handle.gl_id);
    bind("terrain_types", self.types.handle.gl_id);
    if USE_LIGHTING {
      bind("normals", self.normals.handle.gl_id);
    }
  }

  pub fn push(
    &mut self,
    gl: &mut GLContext,
    id: EntityId,
    terrain: &TerrainPiece,
  ) {
    self.id_to_index.insert(id, self.index_to_id.len());
    self.index_to_id.push(id);

    self.length += 3;
    self.vertex_positions.buffer.push(
      gl,
      &[
        terrain.vertices[0].x,
        terrain.vertices[0].y,
        terrain.vertices[0].z,
        terrain.vertices[1].x,
        terrain.vertices[1].y,
        terrain.vertices[1].z,
        terrain.vertices[2].x,
        terrain.vertices[2].y,
        terrain.vertices[2].z,
      ]
    );
    if USE_LIGHTING {
      self.normals.buffer.push(
        gl,
        &[terrain.normal.x, terrain.normal.y, terrain.normal.z]
      );
    }
    self.types.buffer.push(gl, &[terrain.typ as GLuint]);
  }

  // Note: `id` must be present in the buffers.
  pub fn swap_remove(&mut self, gl: &mut GLContext, id: EntityId) {
    let idx = *self.id_to_index.get(&id).unwrap();
    let swapped_id = self.index_to_id[self.index_to_id.len() - 1];
    self.index_to_id.swap_remove(idx).unwrap();
    self.id_to_index.remove(&id);

    if id != swapped_id {
      self.id_to_index.insert(swapped_id, idx);
    }

    self.length -= 3;
    self.vertex_positions.buffer.swap_remove(
      gl,
      idx * 3 * VERTICES_PER_TRIANGLE,
      3 * VERTICES_PER_TRIANGLE
    );
    self.types.buffer.swap_remove(gl, idx, 1);
    if USE_LIGHTING {
      self.normals.buffer.swap_remove(gl, 3 * idx, 3);
    }
  }

  pub fn draw(&self, _gl: &GLContext) {
    unsafe {
      gl::BindVertexArray(self.empty_array.gl_id);
      gl::DrawArrays(gl::TRIANGLES, 0, self.length as GLint);
    }
  }
}
