//! Data structures and functions to load/unload/maintain mob data in VRAM.

use std::collections::hash_map::Entry;
use yaglw::vertex_buffer::{GLArray, GLBuffer, VertexAttribData};
use yaglw::vertex_buffer::{DrawMode, GLType};
use yaglw::gl_context::GLContext;

use common::entity_id;
use common::fnv_map;

use vertex::ColoredVertex;
use view;

/// Number of vertices in a player mesh.
pub const VERTICES_PER_PLAYER: usize = 36;

/// This data structure keeps tracks of mob data in VRAM.
pub struct T<'a> {
  id_to_index: fnv_map::T<entity_id::T, usize>,
  index_to_id: Vec<entity_id::T>,

  triangles: GLArray<'a, ColoredVertex>,
}

#[allow(missing_docs)]
pub fn new<'a, 'b>(
  gl: &'b mut GLContext,
  shader: &view::shaders::color::T<'a>,
) -> T<'a> where
  'a: 'b,
{
  let buffer = GLBuffer::new(gl, 32 * VERTICES_PER_PLAYER);
  T {
    id_to_index: fnv_map::new(),
    index_to_id: Vec::new(),

    triangles: GLArray::new(
      gl,
      &shader.shader,
      &[
        VertexAttribData { name: "position", size: 3, unit: GLType::Float, divisor: 0 },
        VertexAttribData { name: "in_color", size: 4, unit: GLType::Float, divisor: 0 },
      ],
      DrawMode::Triangles,
      buffer,
    ),
  }
}

impl<'a> T<'a> {
  /// Add a single mob into VRAM and return true.
  /// If the mob ID is already loaded, replace the existing mob and return false.
  pub fn insert(
    &mut self,
    gl: &mut GLContext,
    id: entity_id::T,
    triangles: &[ColoredVertex; VERTICES_PER_PLAYER],
  ) -> bool {
    match self.id_to_index.entry(id) {
      Entry::Vacant(entry) => {
        entry.insert(self.index_to_id.len());
        self.index_to_id.push(id);

        self.triangles.buffer.byte_buffer.bind(gl);
        assert!(self.triangles.push(gl, triangles));
        true
      },
      Entry::Occupied(entry) => {
        let idx = *entry.get();
        self.triangles.buffer.byte_buffer.bind(gl);
        self.triangles.buffer.update(gl, idx * VERTICES_PER_PLAYER, triangles);
        false
      },
    }
  }

  /// Draw all the mobs.
  /// N.B. This does not bind any shaders.
  pub fn draw(&self, gl: &mut GLContext) {
    self.triangles.bind(gl);
    self.triangles.draw(gl);
  }
}
