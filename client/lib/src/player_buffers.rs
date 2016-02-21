//! Data structures and functions to load/unload/maintain mob data in VRAM.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use yaglw::vertex_buffer::{GLArray, GLBuffer, VertexAttribData};
use yaglw::vertex_buffer::{DrawMode, GLType};
use yaglw::gl_context::GLContext;

use common::entity_id;

use vertex::ColoredVertex;
use shaders::color::ColorShader;

pub const VERTICES_PER_PLAYER: usize = 36;

/// This data structure keeps tracks of mob data in VRAM.
pub struct PlayerBuffers<'a> {
  id_to_index: HashMap<entity_id::T, usize>,
  index_to_id: Vec<entity_id::T>,

  triangles: GLArray<'a, ColoredVertex>,
}

impl<'a> PlayerBuffers<'a> {
  #[allow(missing_docs)]
  pub fn new<'b>(
    gl: &'b mut GLContext,
    shader: &ColorShader<'a>,
  ) -> Self where
    'a: 'b,
  {
    let buffer = GLBuffer::new(gl, 32 * VERTICES_PER_PLAYER);
    PlayerBuffers {
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
