use common::*;
use glw::gl_buffer::{GLArray, GLBuffer, Triangles};
use glw::gl_context::GLContext;
use glw::shader::Shader;
use glw::vertex;
use nalgebra::Vec3;
use state::App;
use state::EntityId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

// N.B.: Behaviors are unsafe because they take both a mutable and immutable
// reference to a mob (the mob is also inside the main::App).
pub type Behavior = unsafe fn(&App, &mut Mob);

pub struct Mob {
  pub speed: Vec3<f32>,
  pub behavior: Behavior,
  pub id: EntityId,
}

pub struct MobBuffers {
  id_to_index: HashMap<EntityId, uint>,
  index_to_id: Vec<EntityId>,

  triangles: GLArray<vertex::ColoredVertex>,
}

impl MobBuffers {
  pub fn new(gl: &GLContext, color_shader: Rc<RefCell<Shader>>) -> MobBuffers {
    MobBuffers {
      id_to_index: HashMap::new(),
      index_to_id: Vec::new(),

      triangles: GLArray::new(
        gl,
        color_shader.clone(),
        [ vertex::AttribData { name: "position", size: 3, unit: vertex::Float },
          vertex::AttribData { name: "in_color", size: 4, unit: vertex::Float },
        ],
        Triangles,
        GLBuffer::new(32 * TRIANGLE_VERTICES_PER_BOX),
      ),
    }
  }

  pub fn push(
    &mut self,
    id: EntityId,
    triangles: &[vertex::ColoredVertex]
  ) {
    self.id_to_index.insert(id, self.index_to_id.len());
    self.index_to_id.push(id);

    self.triangles.push(triangles);
  }

  pub fn update(
    &mut self,
    id: EntityId,
    triangles: &[vertex::ColoredVertex]
  ) {
    let idx = *self.id_to_index.get(&id).unwrap();
    self.triangles.buffer.update(idx, triangles);
  }

  pub fn draw(&self, gl: &GLContext) {
    self.triangles.draw(gl);
  }
}
