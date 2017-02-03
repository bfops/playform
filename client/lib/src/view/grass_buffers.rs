//! Each tuft of grass in the grass buffer is associated with a terrain polygon. It is associated using
//! an index into the VRAM buffer of terrain polygons.

use cgmath;
use cgmath::{Point3, EuclideanSpace};
use gl;
use std;
use std::f32;
use yaglw;
use yaglw::gl_context::GLContext;

use common::entity_id;
use common::fnv_map;

use terrain_mesh;

// VRAM bytes
const BYTE_BUDGET: usize = 64_000_000;
const TUFT_COST: usize = 8;
const TUFT_BUDGET: usize = BYTE_BUDGET / TUFT_COST;

#[derive(Debug, Clone)]
#[repr(C)]
/// A single tuft of grass to be loaded
pub struct Entry {
  /// Index of a polygon in terrain VRAM buffers that sits under this grass tuft
  pub polygon_idx : u32,
  /// Id of which grass texture to use for a given tuft
  pub tex_id      : u32,
}

/// Struct for loading/unloading/maintaining terrain data in VRAM.
pub struct T<'a> {
  id_to_index: fnv_map::T<entity_id::T, usize>,
  index_to_id: Vec<entity_id::T>,

  to_polygon_id: fnv_map::T<entity_id::T, entity_id::T>,
  of_polygon_id: fnv_map::T<entity_id::T, entity_id::T>,

  gl_array: yaglw::vertex_buffer::ArrayHandle<'a>,
  _instance_vertices: yaglw::vertex_buffer::GLBuffer<'a, Vertex>,
  per_tuft: yaglw::vertex_buffer::GLBuffer<'a, Entry>,
}

struct Vertex {
  /// The position of this vertex in the world.
  pub world_position    : cgmath::Point3<f32>,

  /// The position of this vertex on a texture. The range of valid values
  /// in each dimension is [0, 1].
  pub texture_position  : cgmath::Vector2<f32>,

  /// The translation of the entire model (applied after scaling effects)
  pub model_translation : cgmath::Vector3<f32>,
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
    let normal = cgmath::Vector3::new(0.0, 1.0, 0.0);
    let mut quad = |t: &cgmath::Matrix3<f32>| {
      let mut tri = |mt, p0, t0, p1, t1, p2, t2| {
        let vert = |p, t| {
          Vertex {
            world_position: p,
            model_translation: mt,
            texture_position: t,
          }
        };
        vertices.push(vert(p0, t0));
        vertices.push(vert(p1, t1));
        vertices.push(vert(p2, t2));
      };

      let v = *t * cgmath::Vector3::new(0.0, 0.0, -1.0);
      let model_translation = *t * cgmath::Vector3::new(1.0, 0.0, 0.0) / 2.0;

      tri(
        model_translation,
        Point3::from_vec(-v/2.0)          , cgmath::Vector2::new(0.0 , 0.0) ,
        Point3::from_vec( v/2.0)          , cgmath::Vector2::new(1.0 , 0.0) ,
        Point3::from_vec( v/2.0 + normal) , cgmath::Vector2::new(1.0 , 1.0) ,
      );
      tri(
        model_translation,
        Point3::from_vec(-v/2.0)          , cgmath::Vector2::new(0.0 , 0.0) ,
        Point3::from_vec( v/2.0 + normal) , cgmath::Vector2::new(1.0 , 1.0) ,
        Point3::from_vec(-v/2.0 + normal) , cgmath::Vector2::new(0.0 , 1.0) ,
      );
    };

    use cgmath::SquareMatrix;
    quad(&cgmath::Matrix3::from_value(1.0));
    quad(&cgmath::Matrix3::from_axis_angle(normal, cgmath::Rad(f32::consts::FRAC_PI_3)));
    quad(&cgmath::Matrix3::from_axis_angle(normal, cgmath::Rad(2.0 * f32::consts::FRAC_PI_3)));
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
        vertex_buffer::VertexAttribData {
          name: "model_translation",
          size: 3,
          unit: vertex_buffer::GLType::Float,
          divisor: 0,
        },
      ],
      gl,
      shader,
    );
  assert!(attrib_span == std::mem::size_of::<Vertex>() as u32);

  per_tuft.byte_buffer.bind(gl);
  let attrib_span =
    vertex_buffer::VertexAttribData::apply(
      &[
        vertex_buffer::VertexAttribData {
          name: "polygon_id",
          size: 1,
          unit: vertex_buffer::GLType::Int,
          divisor: 1,
        },
        vertex_buffer::VertexAttribData {
          name: "tex_id",
          size: 1,
          unit: vertex_buffer::GLType::UInt,
          divisor: 1,
        },
      ],
      gl,
      shader,
    );
  assert!(attrib_span == std::mem::size_of::<terrain_mesh::Grass>() as u32);

  T {
    id_to_index: fnv_map::new(),
    index_to_id: Vec::new(),

    to_polygon_id: fnv_map::new(),
    of_polygon_id: fnv_map::new(),

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
    grass: &[Entry],
    polygon_ids: &[entity_id::T],
    grass_ids: &[entity_id::T],
  ) {
    assert!(grass.len() == polygon_ids.len());
    assert!(grass.len() == grass_ids.len());

    self.per_tuft.byte_buffer.bind(gl);
    let success: bool = self.per_tuft.push(gl, grass);
    if !success {
      panic!("Ran out of VRAM for grass");
    }

    for id in grass_ids {
      self.id_to_index.insert(*id, self.index_to_id.len());
      self.index_to_id.push(*id);
    }

    for (id, polygon_id) in grass_ids.iter().zip(polygon_ids.iter()) {
      self.to_polygon_id.insert(*id, *polygon_id);
      self.of_polygon_id.insert(*polygon_id, *id);
    }
  }

  // TODO: Make this take many ids as a parameter, to reduce `bind`s.
  // Note: `id` must be present in the buffers.
  /// Remove some entity from VRAM.
  pub fn swap_remove(&mut self, gl: &mut GLContext, id: entity_id::T) {
    let idx = (*self).id_to_index[&id];
    let swapped_id = self.index_to_id[self.index_to_id.len() - 1];
    self.index_to_id.swap_remove(idx);
    self.id_to_index.remove(&id);

    if id != swapped_id {
      self.id_to_index.insert(swapped_id, idx);
    }

    self.per_tuft.byte_buffer.bind(gl);
    self.per_tuft.swap_remove(gl, idx, 1);

    let polygon_id = self.to_polygon_id.remove(&id).unwrap();
    self.of_polygon_id.remove(&polygon_id).unwrap();
  }

  /// Update the index of the underlying polygon that a grass tuft is associated with.
  pub fn update_polygon_index(
    &self,
    gl: &mut GLContext,
    polygon_id: entity_id::T,
    new_index: u32,
  ) {
    let grass_id =
      match self.of_polygon_id.get(&polygon_id) {
        None => return,
        Some(id) => id,
      };
    let entry_idx = self.id_to_index[grass_id];
    // update the underlying byte buffer directly and only touch the polygon
    // index field.
    self.per_tuft.byte_buffer.bind(gl);
    unsafe {
      self.per_tuft.byte_buffer.update(
        gl,
        std::mem::size_of::<Entry>() * entry_idx,
        &new_index as *const u32 as *const u8,
        std::mem::size_of::<u32>(),
      );
    }
  }

  #[allow(missing_docs)]
  pub fn draw(&self, _gl: &mut GLContext) {
    unsafe {
      gl::BindVertexArray(self.gl_array.gl_id);
      gl::DrawArraysInstanced(gl::TRIANGLES, 0, 18, self.index_to_id.len() as i32);
    }
  }
}
