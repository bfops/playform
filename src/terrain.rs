use common::*;
use gl::types::*;
use glw::gl_buffer::{GLArray, GLBuffer, Triangles};
use glw::gl_context::GLContext;
use glw::shader::Shader;
use glw::vertex;
use main::EntityId;
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
  pub id: EntityId,
}

pub struct TerrainBuffers {
  id_to_index: HashMap<EntityId, uint>,
  index_to_id: Vec<EntityId>,

  triangles: GLArray<TerrainVertex>,
}

impl TerrainBuffers {
  pub fn new(
    gl: &GLContext,
    texture_shader: Rc<RefCell<Shader>>
  ) -> TerrainBuffers {
    TerrainBuffers {
      id_to_index: HashMap::new(),
      index_to_id: Vec::new(),
      triangles: GLArray::new(
        gl,
        texture_shader.clone(),
        [
          vertex::AttribData { name: "position", size: 3, unit: vertex::Float },
          vertex::AttribData { name: "terrain_type", size: 1, unit: vertex::UInt },
        ],
        Triangles,
        GLBuffer::new(MAX_WORLD_SIZE * VERTICES_PER_TRIANGLE),
      ),
    }
  }

  pub fn push(
    &mut self,
    id: EntityId,
    triangles: &[TerrainVertex],
  ) {
    self.id_to_index.insert(id, self.index_to_id.len());
    self.index_to_id.push(id);
    self.triangles.push(triangles);
  }

  // Note: `id` must be present in the buffers.
  pub fn swap_remove(&mut self, id: EntityId) {
    let idx = *self.id_to_index.find(&id).unwrap();
    let swapped_id = self.index_to_id[self.index_to_id.len() - 1];
    self.index_to_id.swap_remove(idx).unwrap();
    self.id_to_index.remove(&id);
    self.triangles.buffer.swap_remove(idx * VERTICES_PER_TRIANGLE, VERTICES_PER_TRIANGLE);
    if id != swapped_id {
      self.id_to_index.insert(swapped_id, idx);
    }
  }

  pub fn draw(&self, gl: &GLContext) {
    self.triangles.draw(gl);
  }
}

