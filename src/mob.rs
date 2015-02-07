use common::*;
use nalgebra::{Pnt3, Vec3};
use shaders::color::ColorShader;
use state::App;
use state::EntityId;
use std::collections::HashMap;
use surroundings_loader::SurroundingsLoader;
use vertex::ColoredVertex;
use yaglw::vertex_buffer::{GLArray, GLBuffer, VertexAttribData};
use yaglw::vertex_buffer::{DrawMode, GLType};
use yaglw::gl_context::GLContext;

pub type Behavior = fn(&App, &mut Mob);

pub struct Mob<'a> {
  pub position: Pnt3<f32>,
  pub speed: Vec3<f32>,
  pub behavior: Behavior,
  pub id: EntityId,

  // Nearby blocks should be made solid if they aren't loaded yet.
  pub solid_boundary: SurroundingsLoader<'a>,
}

pub struct MobBuffers<'a> {
  id_to_index: HashMap<EntityId, usize>,
  index_to_id: Vec<EntityId>,

  triangles: GLArray<'a, ColoredVertex>,
}

impl<'a> MobBuffers<'a> {
  pub fn new<'b:'a>(
    gl: &'a mut GLContext,
    shader: &ColorShader<'b>,
  ) -> MobBuffers<'b> {
    let buffer = GLBuffer::new(gl, 32 * TRIANGLE_VERTICES_PER_BOX as usize);
    MobBuffers {
      id_to_index: HashMap::new(),
      index_to_id: Vec::new(),

      triangles: GLArray::new(
        gl,
        &shader.shader,
        &[
          VertexAttribData { name: "position", size: 3, unit: GLType::Float },
          VertexAttribData { name: "in_color", size: 4, unit: GLType::Float },
        ],
        DrawMode::Triangles,
        buffer,
      ),
    }
  }

  pub fn push(
    &mut self,
    gl: &mut GLContext,
    id: EntityId,
    triangles: &[ColoredVertex]
  ) {
    self.id_to_index.insert(id, self.index_to_id.len());
    self.index_to_id.push(id);

    self.triangles.buffer.byte_buffer.bind(gl);
    self.triangles.push(gl, triangles);
  }

  pub fn update(
    &mut self,
    gl: &mut GLContext,
    id: EntityId,
    triangles: &[ColoredVertex]
  ) {
    let idx = *self.id_to_index.get(&id).unwrap();
    self.triangles.buffer.byte_buffer.bind(gl);
    self.triangles.buffer.update(gl, idx, triangles);
  }

  pub fn draw(&self, gl: &mut GLContext) {
    self.triangles.bind(gl);
    self.triangles.draw(gl);
  }
}
