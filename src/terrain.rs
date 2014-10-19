use common::*;
use gl;
use gl::types::*;
use glw::gl_context::GLContext;
use glw::shader::Shader;
use glw::texture::BufferTexture;
use glw::texture::TextureUnit;
use id_allocator::IdAllocator;
use main::EntityId;
use nalgebra::{Pnt3, Vec3};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

static MAX_WORLD_SIZE: uint = 400000;

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

pub struct TerrainBuffers {
  id_to_index: HashMap<EntityId, uint>,
  index_to_id: Vec<EntityId>,

  empty_array: GLuint,
  length: uint,
  // Each position is buffered as 3 separate floats due to image format restrictions.
  vertex_positions: BufferTexture<GLfloat>,
  // Each normal component is buffered separately floats due to image format restrictions.
  normals: BufferTexture<GLfloat>,
  types: BufferTexture<GLuint>,
}

impl TerrainBuffers {
  pub fn new(
    gl: &GLContext,
  ) -> TerrainBuffers {
    TerrainBuffers {
      id_to_index: HashMap::new(),
      index_to_id: Vec::new(),
      empty_array: unsafe {
        let mut empty_array = 0;
        gl::GenVertexArrays(1, &mut empty_array);
        empty_array
      },
      length: 0,
      // multiply by 3 because there are 3 R32F components
      vertex_positions: BufferTexture::new(gl, gl::R32F, 3 * MAX_WORLD_SIZE * VERTICES_PER_TRIANGLE),
      normals: BufferTexture::new(gl, gl::R32F, 3 * MAX_WORLD_SIZE),
      types: BufferTexture::new(gl, gl::R32UI, MAX_WORLD_SIZE),
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
      gl::ActiveTexture(unit.gl_id());
      gl::BindTexture(gl::TEXTURE_BUFFER, id);
      shader.borrow_mut().with_uniform_location(gl, name, |loc| {
        gl::Uniform1i(loc, unit.glsl_id as GLint);
      });
    };

    bind("positions", self.vertex_positions.texture.gl_id);
    if USE_LIGHTING {
      bind("normals", self.normals.texture.gl_id);
    }
    bind("terrain_types", self.types.texture.gl_id);
  }

  pub fn push(
    &mut self,
    id: EntityId,
    terrain: &TerrainPiece,
  ) {
    self.id_to_index.insert(id, self.index_to_id.len());
    self.index_to_id.push(id);

    self.length += 3;
    self.vertex_positions.buffer.push([
      terrain.vertices[0].x,
      terrain.vertices[0].y,
      terrain.vertices[0].z,
      terrain.vertices[1].x,
      terrain.vertices[1].y,
      terrain.vertices[1].z,
      terrain.vertices[2].x,
      terrain.vertices[2].y,
      terrain.vertices[2].z,
    ]);
    if USE_LIGHTING {
      self.normals.buffer.push([terrain.normal.x, terrain.normal.y, terrain.normal.z]);
    }
    self.types.buffer.push(&[terrain.typ as GLuint]);
  }

  // Note: `id` must be present in the buffers.
  pub fn swap_remove(&mut self, id: EntityId) {
    let idx = *self.id_to_index.find(&id).unwrap();
    let swapped_id = self.index_to_id[self.index_to_id.len() - 1];
    self.index_to_id.swap_remove(idx).unwrap();
    self.id_to_index.remove(&id);

    if id != swapped_id {
      self.id_to_index.insert(swapped_id, idx);
    }

    self.length -= 3;
    self.vertex_positions.buffer.swap_remove(idx * 3 * VERTICES_PER_TRIANGLE, 3 * VERTICES_PER_TRIANGLE);
    if USE_LIGHTING {
      self.normals.buffer.swap_remove(3 * idx, 3);
    }
    self.types.buffer.swap_remove(idx, 1);
  }

  pub fn draw(&self, _gl: &GLContext) {
    gl::BindVertexArray(self.empty_array);
    gl::DrawArrays(gl::TRIANGLES, 0, self.length as GLint);
  }
}

