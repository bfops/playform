use common::*;
use gl::types::*;
use glw::gl_buffer::{GLSliceBuffer, Triangles};
use glw::gl_context::GLContext;
use glw::shader::Shader;
use glw::vertex;
use id_allocator::Id;
use nalgebra::Pnt3;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

static MAX_WORLD_SIZE: uint = 200000;

#[deriving(Show, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TerrainType {
  Grass,
  Dirt,
  Stone,
}

#[deriving(Show, Clone)]
pub struct TerrainVertex {
  pub position: Pnt3<GLfloat>,
  pub terrain_type: GLuint,
}

pub struct TerrainPiece {
  pub vertices: [TerrainVertex, ..3],
  pub id: Id,
}

pub struct TerrainBuffers {
  id_to_index: HashMap<Id, uint>,
  index_to_id: Vec<Id>,

  triangles: GLSliceBuffer<TerrainVertex>,
}

impl TerrainBuffers {
  pub fn new(
      gl: &GLContext,
      texture_shader: Rc<RefCell<Shader>>
  ) -> TerrainBuffers {
    TerrainBuffers {
      id_to_index: HashMap::new(),
      index_to_id: Vec::new(),
      triangles: GLSliceBuffer::new(
        gl,
        texture_shader.clone(),
        [
          vertex::AttribData { name: "position", size: 3, unit: vertex::Float },
          vertex::AttribData { name: "terrain_type", size: 1, unit: vertex::UInt },
        ],
        VERTICES_PER_TRIANGLE,
        MAX_WORLD_SIZE,
        Triangles
      ),
    }
  }

  pub fn push(
    &mut self,
    gl: &GLContext,
    id: Id,
    triangles: &[TerrainVertex],
  ) {
    self.id_to_index.insert(id, self.index_to_id.len());
    self.index_to_id.push(id);
    self.triangles.push(gl, triangles);
  }

  // Note: `id` must be present in the buffers.
  pub fn swap_remove(&mut self, gl: &GLContext, id: Id) {
    let idx = *self.id_to_index.find(&id).unwrap();
    let swapped_id = self.index_to_id[self.index_to_id.len() - 1];
    self.index_to_id.swap_remove(idx).unwrap();
    self.id_to_index.remove(&id);
    self.triangles.swap_remove(gl, idx);
    if id != swapped_id {
      self.id_to_index.insert(swapped_id, idx);
    }
  }

  pub fn draw(&self, gl: &GLContext) {
    self.triangles.draw(gl);
  }
}

