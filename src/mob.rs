use common::*;
use main;
use glw::gl_buffer::{GLSliceBuffer, Triangles};
use glw::gl_context::GLContext;
use glw::shader::Shader;
use glw::vertex;
use nalgebra::na::Vec3;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

type Behavior = fn(&main::App, &mut Mob);

pub struct Mob {
  pub speed: Vec3<f32>,
  pub behavior: Behavior,
  pub id: main::Id,
}

pub struct MobBuffers {
  id_to_index: HashMap<main::Id, uint>,
  index_to_id: Vec<main::Id>,

  triangles: GLSliceBuffer<vertex::ColoredVertex>,
}

impl MobBuffers {
  pub unsafe fn new(gl: &GLContext, color_shader: &Rc<RefCell<Shader>>) -> MobBuffers {
    MobBuffers {
      id_to_index: HashMap::new(),
      index_to_id: Vec::new(),

      triangles: GLSliceBuffer::new(
        gl,
        color_shader.clone(),
        [ vertex::AttribData { name: "position", size: 3 },
          vertex::AttribData { name: "in_color", size: 4 },
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
    id: main::Id,
    triangles: &[vertex::ColoredVertex]
  ) {
    self.id_to_index.insert(id, self.index_to_id.len());
    self.index_to_id.push(id);

    self.triangles.push(gl, triangles);
  }

  pub fn update(
    &mut self,
    gl: &GLContext,
    id: main::Id,
    triangles: &[vertex::ColoredVertex]
  ) {
    let idx = *unwrap!(self.id_to_index.find(&id));
    self.triangles.update(gl, idx, triangles);
  }

  pub fn draw(&self, gl: &GLContext) {
    self.triangles.draw(gl);
  }
}

