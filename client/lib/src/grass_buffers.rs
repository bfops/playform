use std;
use std::collections::HashMap;
use yaglw;
use yaglw::gl_context::GLContext;

use common::entity_id;

use terrain_mesh::Triangle;
use vertex;

#[cfg(test)]
use std::mem;

// VRAM bytes
pub const BYTE_BUDGET: usize = 64_000_000;
pub const POLYGON_COST: usize = 100;
pub const POLYGON_BUDGET: usize = BYTE_BUDGET / POLYGON_COST;

/// Struct for loading/unloading/maintaining terrain data in VRAM.
pub struct T<'a> {
  id_to_index: HashMap<entity_id::T, usize>,
  index_to_id: Vec<entity_id::T>,

  vertices: yaglw::vertex_buffer::GLArray<'a, vertex::TextureVertex>,
}

#[allow(missing_docs)]
pub fn new<'a, 'b:'a>(
  gl: &'a mut GLContext,
  shader: &yaglw::shader::Shader<'b>,
) -> T<'b>
{
  use yaglw::vertex_buffer;

  T {
    id_to_index: HashMap::new(),
    index_to_id: Vec::new(),

    vertices: {
      let buffer = vertex_buffer::GLBuffer::new(gl, POLYGON_BUDGET * 3);
      vertex_buffer::GLArray::new(
        gl,
        shader,
        &[
          vertex_buffer::VertexAttribData { name: "position"         , size: 3 , unit: vertex_buffer::GLType::Float } ,
          vertex_buffer::VertexAttribData { name: "texture_position" , size: 2 , unit: vertex_buffer::GLType::Float } ,
        ],
        vertex_buffer::DrawMode::Triangles,
        buffer,
      )
    },
  }
}

impl<'a> T<'a> {
  /// Add a series of entites into VRAM.
  pub fn push(
    &mut self,
    gl: &mut GLContext,
    vertices: &[Triangle<vertex::TextureVertex>],
    ids: &[entity_id::T],
  ) {
    assert_eq!(vertices.len(), ids.len());

    self.vertices.buffer.byte_buffer.bind(gl);
    let vertices =
      unsafe {
        let vertices: std::raw::Slice<vertex::TextureVertex> = std::mem::transmute(vertices);
        std::slice::from_raw_parts(vertices.data, vertices.len * 3)
      };
    let success: bool = self.vertices.push(gl, vertices);
    if !success {
      panic!("Ran out of VRAM for grass");
    }

    for &id in ids.iter() {
      self.id_to_index.insert(id, self.index_to_id.len());
      self.index_to_id.push(id);
    }
  }

  // TODO: Make this take many ids as a parameter, to reduce `bind`s.
  // Note: `id` must be present in the buffers.
  /// Remove some entity from VRAM.
  pub fn swap_remove(&mut self, gl: &mut GLContext, id: entity_id::T) {
    let idx = *self.id_to_index.get(&id).unwrap();
    let swapped_id = self.index_to_id[self.index_to_id.len() - 1];
    self.index_to_id.swap_remove(idx);
    self.id_to_index.remove(&id);

    if id != swapped_id {
      self.id_to_index.insert(swapped_id, idx);
    }

    self.vertices.buffer.byte_buffer.bind(gl);
    self.vertices.swap_remove(gl, idx * 3, 3);
  }

  /// Draw the terrain.
  pub fn draw(&self, gl: &mut GLContext) {
    self.vertices.bind(gl);
    self.vertices.draw(gl);
  }
}
