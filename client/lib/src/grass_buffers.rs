use cgmath;
use cgmath::{Vector, Point, Matrix};
use gl;
use std;
use std::collections::HashMap;
use std::f32;
use yaglw;
use yaglw::gl_context::GLContext;

use common::entity_id;

use terrain_mesh;
use vertex;

// VRAM bytes
pub const BYTE_BUDGET: usize = 64_000_000;
pub const TUFT_COST: usize = 64;
pub const TUFT_BUDGET: usize = BYTE_BUDGET / TUFT_COST;

/// Struct for loading/unloading/maintaining terrain data in VRAM.
pub struct T<'a> {
  id_to_index: HashMap<entity_id::T, usize>,
  index_to_id: Vec<entity_id::T>,

  gl_array: yaglw::vertex_buffer::ArrayHandle<'a>,
  _instance_vertices: yaglw::vertex_buffer::GLBuffer<'a, vertex::TextureVertex>,
  per_tuft: yaglw::vertex_buffer::GLBuffer<'a, terrain_mesh::Grass>,
}

#[allow(missing_docs)]
pub fn new<'a, 'b:'a>(
  gl: &'a mut GLContext,
  shader: &yaglw::shader::Shader<'a>,
) -> T<'b>
{
  use yaglw::vertex_buffer;

  let gl_array = vertex_buffer::ArrayHandle::new(gl);
  let mut instance_vertices = vertex_buffer::GLBuffer::new(gl, 18);
  let per_tuft = vertex_buffer::GLBuffer::new(gl, TUFT_BUDGET);

  let mut vertices = Vec::new();
  {
    let tangent = cgmath::Vector3::new(0.0, 0.0, 0.5);
    let normal = cgmath::Vector3::new(0.0, 1.0, 0.0);
    let mut quad = |v: &cgmath::Vector3<f32>| {
      let mut tri = |p0, t0, p1, t1, p2, t2| {
        let vert = |p, t| {
          vertex::TextureVertex {
            world_position: p,
            texture_position: t,
          }
        };
        vertices.push(vert(p0, t0));
        vertices.push(vert(p1, t1));
        vertices.push(vert(p2, t2));
      };

      tri(
        Point::from_vec(&-v.div_s(2.0))                , cgmath::Vector2::new(0.0 , 0.0) ,
        Point::from_vec(&v.div_s(2.0))                 , cgmath::Vector2::new(1.0 , 0.0) ,
        Point::from_vec(&v.div_s(2.0).add_v(&normal))  , cgmath::Vector2::new(1.0 , 1.0) ,
      );
      tri(
        Point::from_vec(&-v.div_s(2.0))                , cgmath::Vector2::new(0.0 , 0.0) ,
        Point::from_vec(&v.div_s(2.0).add_v(&normal))  , cgmath::Vector2::new(1.0 , 1.0) ,
        Point::from_vec(&-v.div_s(2.0).add_v(&normal)) , cgmath::Vector2::new(0.0 , 1.0) ,
      );
    };

    quad(&tangent);
    quad(&cgmath::Matrix3::from_axis_angle(&normal, cgmath::rad(f32::consts::FRAC_PI_3)).mul_v(&tangent));
    quad(&cgmath::Matrix3::from_axis_angle(&normal, cgmath::rad(2.0 * f32::consts::FRAC_PI_3)).mul_v(&tangent));
  }

  unsafe {
    gl::BindVertexArray(gl_array.gl_id);
  }
  shader.use_shader(gl);
  instance_vertices.byte_buffer.bind(gl);
  instance_vertices.push(gl, &vertices);
  let attrib_span =
    vertex_buffer::VertexAttribData::apply(
      &[
        vertex_buffer::VertexAttribData {
          name: "vertex_position",
          size: 3,
          unit: vertex_buffer::GLType::Float,
          divisor: 0,
        },
        vertex_buffer::VertexAttribData {
          name: "texture_position",
          size: 2,
          unit: vertex_buffer::GLType::Float,
          divisor: 0,
        },
      ],
      gl,
      shader,
    );
  assert!(attrib_span == std::mem::size_of::<vertex::TextureVertex>() as u32);

  per_tuft.byte_buffer.bind(gl);
  let attrib_span =
    vertex_buffer::VertexAttribData::apply(
      &[
        vertex_buffer::VertexAttribData {
          name: "root",
          size: 3,
          unit: vertex_buffer::GLType::Float,
          divisor: 1,
        },
        vertex_buffer::VertexAttribData {
          name: "normal",
          size: 3,
          unit: vertex_buffer::GLType::Float,
          divisor: 1,
        },
        vertex_buffer::VertexAttribData {
          name: "tex_id",
          size: 3,
          unit: vertex_buffer::GLType::UInt,
          divisor: 1,
        },
      ],
      gl,
      shader,
    );
  assert!(attrib_span == std::mem::size_of::<terrain_mesh::Grass>() as u32);

  T {
    id_to_index: HashMap::new(),
    index_to_id: Vec::new(),

    gl_array: gl_array,
    _instance_vertices: instance_vertices,
    per_tuft: per_tuft,
  }
}

impl<'a> T<'a> {
  /// Add a series of entites into VRAM.
  pub fn push(
    &mut self,
    gl: &mut GLContext,
    grass: &[terrain_mesh::Grass],
    grass_ids: &[entity_id::T],
  ) {
    self.per_tuft.byte_buffer.bind(gl);
    let success: bool = self.per_tuft.push(gl, grass);
    if !success {
      panic!("Ran out of VRAM for grass");
    }

    for id in grass_ids {
      self.id_to_index.insert(*id, self.index_to_id.len());
      self.index_to_id.push(*id);
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

    self.per_tuft.byte_buffer.bind(gl);
    self.per_tuft.swap_remove(gl, idx, 1);
  }

  pub fn draw(&self, _gl: &mut GLContext) {
    unsafe {
      gl::BindVertexArray(self.gl_array.gl_id);
      gl::DrawArraysInstanced(gl::TRIANGLES, 0, 18, self.index_to_id.len() as i32);
    }
  }
}
