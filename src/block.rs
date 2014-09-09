use common::*;
use main;
use gl::types::*;
use glw::gl_buffer::{GLSliceBuffer, Triangles, Lines};
use glw::gl_context::GLContext;
use glw::shader::Shader;
use glw::vertex;
use nalgebra::na::{Vec2, Vec3};
use ncollide3df32::bounding_volume::LooseBoundingVolume;
use ncollide3df32::bounding_volume::aabb::AABB;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

static MAX_WORLD_SIZE: uint = 100000;

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
  pub fn texture_positions() -> Vec<Vec2<GLfloat>> {
    // hacky little solution so that we don't index right onto the edge of a
    // texture; if we do, we get edges showing up in rendering.
    let d = 0.01;

    Vec::from_slice([
      // front
      Vec2::new(0.00 + d, 0.50 + d),
      Vec2::new(0.25 - d, 0.75 - d),
      Vec2::new(0.25 - d, 0.50 + d),
      Vec2::new(0.00 + d, 0.50 + d),
      Vec2::new(0.00 + d, 0.75 - d),
      Vec2::new(0.25 - d, 0.75 - d),
      // left
      Vec2::new(0.75 - d, 1.00 - d),
      Vec2::new(0.50 + d, 0.75 + d),
      Vec2::new(0.50 + d, 1.00 - d),
      Vec2::new(0.75 - d, 1.00 - d),
      Vec2::new(0.75 - d, 0.75 + d),
      Vec2::new(0.50 + d, 0.75 + d),
      // top
      Vec2::new(0.25 + d, 0.75 - d),
      Vec2::new(0.50 - d, 0.50 + d),
      Vec2::new(0.25 + d, 0.50 + d),
      Vec2::new(0.25 + d, 0.75 - d),
      Vec2::new(0.50 - d, 0.75 - d),
      Vec2::new(0.50 - d, 0.50 + d),
      // back
      Vec2::new(0.75 - d, 0.50 + d),
      Vec2::new(0.50 + d, 0.75 - d),
      Vec2::new(0.75 - d, 0.75 - d),
      Vec2::new(0.75 - d, 0.50 + d),
      Vec2::new(0.50 + d, 0.50 + d),
      Vec2::new(0.50 + d, 0.75 - d),
      // right
      Vec2::new(0.75 - d, 0.25 + d),
      Vec2::new(0.50 + d, 0.50 - d),
      Vec2::new(0.75 - d, 0.50 - d),
      Vec2::new(0.75 - d, 0.25 + d),
      Vec2::new(0.50 + d, 0.25 + d),
      Vec2::new(0.50 + d, 0.50 - d),
      // bottom
      Vec2::new(0.75 + d, 0.50 + d),
      Vec2::new(1.00 - d, 0.75 - d),
      Vec2::new(1.00 - d, 0.50 + d),
      Vec2::new(0.75 + d, 0.50 + d),
      Vec2::new(0.75 + d, 0.75 - d),
      Vec2::new(1.00 - d, 0.75 - d),
    ])
  }

  pub fn vertex_normals() -> Vec<Vec3<GLfloat>> {
    Vec::from_slice([
      Vec3::new( 0.0,  0.0,  1.0),
      Vec3::new(-1.0,  0.0,  0.0),
      Vec3::new( 0.0,  1.0,  0.0),
      Vec3::new( 0.0,  0.0, -1.0),
      Vec3::new( 1.0,  0.0,  0.0),
      Vec3::new( 0.0, -1.0,  0.0),
    ])
  }

  // Construct outlines for this Block, to sharpen the edges.
  pub fn to_outlines(&self, bounds: &AABB) -> [vertex::ColoredVertex, ..LINE_VERTICES_PER_BOX] {
    to_outlines(&bounds.loosened(0.002))
  }

  pub fn to_texture_triangles(&self, bounds: &AABB) -> [BlockVertex, ..TRIANGLE_VERTICES_PER_BOX] {
    let (x1, y1, z1) = (bounds.mins().x, bounds.mins().y, bounds.mins().z);
    let (x2, y2, z2) = (bounds.maxs().x, bounds.maxs().y, bounds.maxs().z);

    let vtx = |x, y, z| {
      BlockVertex {
        position: Vec3::new(x, y, z),
        block_type: self.block_type as GLuint,
      }
    };

    // Remember: x increases to the right, y increases up, and z becomes more
    // negative as depth from the viewer increases.
    [
      // front
      vtx(x1, y1, z2), vtx(x2, y2, z2), vtx(x1, y2, z2),
      vtx(x1, y1, z2), vtx(x2, y1, z2), vtx(x2, y2, z2),
      // left
      vtx(x1, y1, z1), vtx(x1, y2, z2), vtx(x1, y2, z1),
      vtx(x1, y1, z1), vtx(x1, y1, z2), vtx(x1, y2, z2),
      // top
      vtx(x1, y2, z1), vtx(x2, y2, z2), vtx(x2, y2, z1),
      vtx(x1, y2, z1), vtx(x1, y2, z2), vtx(x2, y2, z2),
      // back
      vtx(x1, y1, z1), vtx(x2, y2, z1), vtx(x2, y1, z1),
      vtx(x1, y1, z1), vtx(x1, y2, z1), vtx(x2, y2, z1),
      // right
      vtx(x2, y1, z1), vtx(x2, y2, z2), vtx(x2, y1, z2),
      vtx(x2, y1, z1), vtx(x2, y2, z1), vtx(x2, y2, z2),
      // bottom
      vtx(x1, y1, z1), vtx(x2, y1, z2), vtx(x1, y1, z2),
      vtx(x1, y1, z1), vtx(x2, y1, z1), vtx(x2, y1, z2),
    ]
  }
}

#[deriving(Show, Clone)]
pub struct BlockVertex {
  pub position: Vec3<GLfloat>,
  pub block_type: GLuint,
}

pub struct BlockBuffers {
  id_to_index: HashMap<main::Id, uint>,
  index_to_id: Vec<main::Id>,

  triangles: GLSliceBuffer<BlockVertex>,
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
        [
          vertex::AttribData { name: "position", size: 3, unit: vertex::Float },
          vertex::AttribData { name: "block_type", size: 1, unit: vertex::UInt },
        ],
        TRIANGLE_VERTICES_PER_BOX,
        MAX_WORLD_SIZE,
        Triangles
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
    triangles: &[BlockVertex],
    outlines: &[vertex::ColoredVertex]
  ) {
    self.id_to_index.insert(id, self.index_to_id.len());
    self.index_to_id.push(id);
    self.triangles.push(gl, triangles);
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

