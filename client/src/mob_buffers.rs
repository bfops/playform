//! Data structures and functions to load/unload/maintain mob data in VRAM.

use common::entity::EntityId;
use common::vertex::ColoredVertex;
use shaders::color::ColorShader;
use std::collections::HashMap;
use yaglw::vertex_buffer::{GLArray, GLBuffer, VertexAttribData};
use yaglw::vertex_buffer::{DrawMode, GLType};
use yaglw::gl_context::GLContext;

const TRIANGLES_PER_BOX: u32 = 12;
const VERTICES_PER_TRIANGLE: u32 = 3;
const TRIANGLE_VERTICES_PER_BOX: u32 = TRIANGLES_PER_BOX * VERTICES_PER_TRIANGLE;

/// This data structure keeps tracks of mob data in VRAM.
pub struct MobBuffers<'a> {
  id_to_index: HashMap<EntityId, usize>,
  index_to_id: Vec<EntityId>,

  triangles: GLArray<'a, ColoredVertex>,
}

impl<'a> MobBuffers<'a> {
  #[allow(missing_docs)]
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

  /// Add a single mob into VRAM.
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

  /// Update an existing mob in VRAM.
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

  /// Draw all the mobs.
  /// N.B. This does not bind any shaders.
  pub fn draw(&self, gl: &mut GLContext) {
    self.triangles.bind(gl);
    self.triangles.draw(gl);
  }
}
