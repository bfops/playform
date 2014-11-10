use common::*;
use glw::vertex_buffer::*;
use glw::gl_context::{GLContext, GLContextExistence};
use glw::shader::Shader;
use nalgebra::Vec3;
use state::App;
use state::EntityId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use vertex::ColoredVertex;

// N.B.: Behaviors are unsafe because they take both a mutable and immutable
// reference to a mob (the mob is also inside the main::App).
pub type Behavior = unsafe fn(&App, &mut Mob);

pub struct Mob {
  pub speed: Vec3<f32>,
  pub behavior: Behavior,
  pub id: EntityId,
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
        [
          VertexAttribData { name: "position", size: 3, unit: Float },
          VertexAttribData { name: "in_color", size: 4, unit: Float },
        ],
        Triangles,
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

    self.triangles.push(gl, triangles);
  }

  pub fn update(
    &mut self,
    gl: &mut GLContext,
    id: EntityId,
    triangles: &[ColoredVertex]
  ) {
    let idx = *self.id_to_index.get(&id).unwrap();
    self.triangles.buffer.update(gl, idx, triangles);
  }

  pub fn draw(&self, gl: &mut GLContext) {
    self.triangles.draw(gl);
  }
}
