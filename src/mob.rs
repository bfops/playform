use common::*;
use nalgebra::{Pnt3, Vec3};
use state::App;
use state::EntityId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use surroundings_loader::SurroundingsLoader;
use vertex::ColoredVertex;
use yaglw::vertex_buffer::{GLArray, GLBuffer, VertexAttribData};
use yaglw::vertex_buffer::{DrawMode, GLType};
use yaglw::gl_context::{GLContext, GLContextExistence};
use yaglw::shader::Shader;

pub type Behavior = fn(&App, &mut Mob);

pub struct Mob<'a> {
  pub position: Pnt3<f32>,
  pub speed: Vec3<f32>,
  pub behavior: Behavior,
  pub id: EntityId,
  pub surroundings_loader: SurroundingsLoader<'a>,
}

pub struct MobBuffers<'a> {
  id_to_index: HashMap<EntityId, uint>,
  index_to_id: Vec<EntityId>,

  triangles: GLArray<'a, ColoredVertex>,
}

impl<'a> MobBuffers<'a> {
  pub fn new(
    gl: &'a GLContextExistence,
    gl_context: &mut GLContext,
    color_shader: Rc<RefCell<Shader>>,
  ) -> MobBuffers<'a> {
    let buffer = GLBuffer::new(gl, gl_context, 32 * TRIANGLE_VERTICES_PER_BOX);
    MobBuffers {
      id_to_index: HashMap::new(),
      index_to_id: Vec::new(),

      triangles: GLArray::new(
        gl,
        gl_context,
        color_shader.clone(),
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
