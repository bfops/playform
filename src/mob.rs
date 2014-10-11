use common::*;
use main;
use glw::gl_buffer::{GLSliceBuffer, Triangles};
use glw::gl_context::GLContext;
use glw::shader::Shader;
use glw::vertex;
use id_allocator::Id;
use nalgebra::Vec3;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

// N.B.: Behaviors are unsafe because they take both a mutable and immutable
// reference to a mob (the mob is also inside the main::App).
pub type Behavior = unsafe fn(&main::App, &mut Mob);

pub struct Mob {
  pub speed: Vec3<f32>,
  pub behavior: Behavior,
  pub id: Id,
}

pub struct MobBuffers {
  id_to_index: HashMap<Id, uint>,
  index_to_id: Vec<Id>,

  triangles: GLSliceBuffer<vertex::ColoredVertex>,
}

impl MobBuffers {
  pub fn new(gl: &GLContext, color_shader: Rc<RefCell<Shader>>) -> MobBuffers {
    MobBuffers {
      id_to_index: HashMap::new(),
      index_to_id: Vec::new(),

      triangles: GLSliceBuffer::new(
        gl,
        color_shader.clone(),
        [ vertex::AttribData { name: "position", size: 3, unit: vertex::Float },
          vertex::AttribData { name: "in_color", size: 4, unit: vertex::Float },
        ],
        TRIANGLE_VERTICES_PER_BOX,
        32,
        Triangles
      ),
    }
  }

  pub fn push(
    &mut self,
    gl: &GLContext,
    id: Id,
    triangles: &[vertex::ColoredVertex]
  ) {
    self.id_to_index.insert(id, self.index_to_id.len());
    self.index_to_id.push(id);

    self.triangles.push(gl, triangles);
  }

  pub fn update(
    &mut self,
    gl: &GLContext,
    id: Id,
    triangles: &[vertex::ColoredVertex]
  ) {
    let idx = *unwrap!(self.id_to_index.find(&id));
    self.triangles.update(gl, idx, triangles);
  }

  pub fn draw(&self, gl: &GLContext) {
    self.triangles.draw(gl);
  }
}

