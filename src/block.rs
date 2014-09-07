use common::*;
use main;
use gl::types::GLfloat;
use glw::gl_buffer::{GLSliceBuffer, Points, Lines};
use glw::gl_context::GLContext;
use glw::shader::Shader;
use glw::vertex;
use nalgebra::na::Vec3;
use ncollide3df32::bounding_volume::LooseBoundingVolume;
use ncollide3df32::bounding_volume::aabb::AABB;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

static MAX_WORLD_SIZE: uint = 40000;

#[deriving(Show, Copy, Clone, PartialEq, Eq, Hash)]
pub enum BlockType {
  Grass,
  Dirt,
  Stone,
}

#[deriving(Clone)]
/// A voxel-ish block in the game world.
pub struct Block {
  pub block_type: BlockType,
  pub id: main::Id,
}

impl Block {
  // Construct outlines for this Block, to sharpen the edges.
  pub fn to_outlines(bounds: &AABB) -> [vertex::ColoredVertex, ..LINE_VERTICES_PER_BOX] {
    to_outlines(&bounds.loosened(0.002))
  }
}

pub struct BlockBuffers {
  id_to_index: HashMap<main::Id, uint>,
  index_to_id: Vec<main::Id>,

  triangles: GLSliceBuffer<Vec3<GLfloat>>,
  outlines: GLSliceBuffer<vertex::ColoredVertex>,
}

impl BlockBuffers {
  pub fn new(
      gl: &GLContext,
      color_shader: &Rc<RefCell<Shader>>,
      texture_shader: &Rc<RefCell<Shader>>
  ) -> BlockBuffers {
    BlockBuffers {
      id_to_index: HashMap::new(),
      index_to_id: Vec::new(),
      triangles: GLSliceBuffer::new(
        gl,
        texture_shader.clone(),
        [ vertex::AttribData { name: "position", size: 3, unit: vertex::Float },
        ],
        1,
        MAX_WORLD_SIZE,
        Points,
      ),
      outlines: GLSliceBuffer::new(
        gl,
        color_shader.clone(),
        [ vertex::AttribData { name: "position", size: 3, unit: vertex::Float },
          vertex::AttribData { name: "in_color", size: 4, unit: vertex::Float },
        ],
        LINE_VERTICES_PER_BOX,
        MAX_WORLD_SIZE,
        Lines
      ),
    }
  }

  pub fn push(
    &mut self,
    gl: &GLContext,
    id: main::Id,
    low_corner: Vec3<GLfloat>,
    outlines: &[vertex::ColoredVertex]
  ) {
    self.id_to_index.insert(id, self.index_to_id.len());
    self.index_to_id.push(id);
    self.triangles.push(gl, &[low_corner]);
    self.outlines.push(gl, outlines);
  }

  pub fn swap_remove(&mut self, gl: &GLContext, id: main::Id) {
    let idx = *unwrap!(self.id_to_index.find(&id));
    let swapped_id = self.index_to_id[self.index_to_id.len() - 1];
    unwrap!(self.index_to_id.swap_remove(idx));
    self.id_to_index.remove(&id);
    self.triangles.swap_remove(gl, idx);
    self.outlines.swap_remove(gl, idx);
    if id != swapped_id {
      self.id_to_index.insert(swapped_id, idx);
    }
  }

  pub fn draw(&self, gl: &GLContext, draw_outlines: bool) {
    self.triangles.draw(gl);
    if draw_outlines {
      self.outlines.draw(gl);
    }
  }
}

