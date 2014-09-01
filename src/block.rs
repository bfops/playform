use common::*;
use main;
use glw::gl_buffer::{GLSliceBuffer, Triangles, Lines};
use glw::gl_context::GLContext;
use glw::shader::Shader;
use glw::vertex;
use nalgebra::na::{Vec2, Vec3};
use ncollide3df32::bounding_volume::LooseBoundingVolume;
use ncollide3df32::bounding_volume::aabb::AABB;
use std::collections::HashMap;
use std::rc::Rc;

static MAX_WORLD_SIZE: uint = 40000;

#[deriving(Copy, Clone, PartialEq, Eq, Hash)]
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

  pub fn to_texture_triangles(bounds: &AABB) -> [vertex::WorldTextureVertex, ..TRIANGLE_VERTICES_PER_BOX] {
    let (x1, y1, z1) = (bounds.mins().x, bounds.mins().y, bounds.mins().z);
    let (x2, y2, z2) = (bounds.maxs().x, bounds.maxs().y, bounds.maxs().z);

    let vtx = |x, y, z, nx, ny, nz, tx, ty| {
      vertex::WorldTextureVertex {
        world_position: Vec3::new(x, y, z),
        texture_position: Vec2::new(tx, ty),
        normal: Vec3::new(nx, ny, nz),
      }
    };

    // hacky little solution so that we don't index right onto the edge of a
    // texture; if we do, we get edges showing up in rendering.
    let d = 0.01;

    // Remember: x increases to the right, y increases up, and z becomes more
    // negative as depth from the viewer increases.
    [
      // front
      vtx(x1, y1, z2,  0.0,  0.0,  1.0, 0.00 + d, 0.50 - d),
      vtx(x2, y2, z2,  0.0,  0.0,  1.0, 0.25 - d, 0.25 + d),
      vtx(x1, y2, z2,  0.0,  0.0,  1.0, 0.25 - d, 0.50 - d),
      vtx(x1, y1, z2,  0.0,  0.0,  1.0, 0.00 + d, 0.50 - d),
      vtx(x2, y1, z2,  0.0,  0.0,  1.0, 0.00 + d, 0.25 + d),
      vtx(x2, y2, z2,  0.0,  0.0,  1.0, 0.25 - d, 0.25 + d),
      // left
      vtx(x1, y1, z1, -1.0,  0.0,  0.0, 0.75 - d, 0.00 + d),
      vtx(x1, y2, z2, -1.0,  0.0,  0.0, 0.50 + d, 0.25 - d),
      vtx(x1, y2, z1, -1.0,  0.0,  0.0, 0.50 + d, 0.00 + d),
      vtx(x1, y1, z1, -1.0,  0.0,  0.0, 0.75 - d, 0.00 + d),
      vtx(x1, y1, z2, -1.0,  0.0,  0.0, 0.75 - d, 0.25 - d),
      vtx(x1, y2, z2, -1.0,  0.0,  0.0, 0.50 + d, 0.25 - d),
      // top
      vtx(x1, y2, z1,  0.0,  1.0,  0.0, 0.25 + d, 0.25 + d),
      vtx(x2, y2, z2,  0.0,  1.0,  0.0, 0.50 - d, 0.50 - d),
      vtx(x2, y2, z1,  0.0,  1.0,  0.0, 0.25 + d, 0.50 - d),
      vtx(x1, y2, z1,  0.0,  1.0,  0.0, 0.25 + d, 0.25 + d),
      vtx(x1, y2, z2,  0.0,  1.0,  0.0, 0.50 - d, 0.25 + d),
      vtx(x2, y2, z2,  0.0,  1.0,  0.0, 0.50 - d, 0.50 - d),
      // back
      vtx(x1, y1, z1,  0.0,  0.0, -1.0, 0.75 - d, 0.50 - d),
      vtx(x2, y2, z1,  0.0,  0.0, -1.0, 0.50 + d, 0.25 + d),
      vtx(x2, y1, z1,  0.0,  0.0, -1.0, 0.75 - d, 0.25 + d),
      vtx(x1, y1, z1,  0.0,  0.0, -1.0, 0.75 - d, 0.50 - d),
      vtx(x1, y2, z1,  0.0,  0.0, -1.0, 0.50 + d, 0.50 - d),
      vtx(x2, y2, z1,  0.0,  0.0, -1.0, 0.50 + d, 0.25 + d),
      // right
      vtx(x2, y1, z1,  1.0,  0.0,  0.0, 0.75 - d, 0.75 - d),
      vtx(x2, y2, z2,  1.0,  0.0,  0.0, 0.50 + d, 0.50 + d),
      vtx(x2, y1, z2,  1.0,  0.0,  0.0, 0.75 - d, 0.50 + d),
      vtx(x2, y1, z1,  1.0,  0.0,  0.0, 0.75 - d, 0.75 - d),
      vtx(x2, y2, z1,  1.0,  0.0,  0.0, 0.50 + d, 0.75 - d),
      vtx(x2, y2, z2,  1.0,  0.0,  0.0, 0.50 + d, 0.50 + d),
      // bottom
      vtx(x1, y1, z1,  0.0, -1.0,  0.0, 0.75 + d, 0.50 - d),
      vtx(x2, y1, z2,  0.0, -1.0,  0.0, 1.00 - d, 0.25 + d),
      vtx(x1, y1, z2,  0.0, -1.0,  0.0, 1.00 - d, 0.50 - d),
      vtx(x1, y1, z1,  0.0, -1.0,  0.0, 0.75 + d, 0.50 - d),
      vtx(x2, y1, z1,  0.0, -1.0,  0.0, 0.75 + d, 0.25 + d),
      vtx(x2, y1, z2,  0.0, -1.0,  0.0, 1.00 - d, 0.25 + d),
    ]
  }
}

pub struct BlockBuffers {
  id_to_index: HashMap<main::Id, uint>,
  index_to_id: Vec<main::Id>,

  triangles: GLSliceBuffer<vertex::WorldTextureVertex>,
  outlines: GLSliceBuffer<vertex::ColoredVertex>,
}

impl BlockBuffers {
  pub unsafe fn new(gl: &GLContext, color_shader: &Rc<Shader>, texture_shader: &Rc<Shader>) -> BlockBuffers {
    BlockBuffers {
      id_to_index: HashMap::new(),
      index_to_id: Vec::new(),
      triangles: GLSliceBuffer::new(
        gl,
        texture_shader.clone(),
        [ vertex::AttribData { name: "position", size: 3 },
          vertex::AttribData { name: "texture_position", size: 2 },
          vertex::AttribData { name: "vertex_normal", size: 3 },
        ],
        TRIANGLE_VERTICES_PER_BOX,
        MAX_WORLD_SIZE,
        Triangles
      ),
      outlines: GLSliceBuffer::new(
        gl,
        color_shader.clone(),
        [ vertex::AttribData { name: "position", size: 3 },
          vertex::AttribData { name: "in_color", size: 4 },
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
    triangles: &[vertex::WorldTextureVertex],
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

  pub fn draw(&self, gl: &GLContext) {
    self.triangles.draw(gl);
    self.outlines.draw(gl);
  }
}

