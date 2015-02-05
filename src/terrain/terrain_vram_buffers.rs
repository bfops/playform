use common::VERTICES_PER_TRIANGLE;
use color::Color3;
use gl;
use gl::types::*;
use id_allocator::IdAllocator;
use nalgebra::{Pnt2, Pnt3, Vec3};
use shaders::terrain::TerrainShader;
use state::EntityId;
use std::collections::HashMap;
use std::iter::{IteratorExt, repeat};
use std::u32;
use terrain::terrain_block::BlockPosition;
use terrain::texture_generator;
use yaglw::gl_context::{GLContext,GLContextExistence};
use yaglw::texture::BufferTexture;
use yaglw::texture::TextureUnit;

#[cfg(test)]
use std::mem;

// VRAM bytes
pub const BYTE_BUDGET: usize = 64_000_000;
pub const POLYGON_COST: usize = 100;
// This assumes there exists only one set of TerrainVRAMBuffers.
pub const POLYGON_BUDGET: usize = BYTE_BUDGET / POLYGON_COST;

pub type Triangle<T> = [T; 3];

pub struct TerrainVRAMBuffers<'a> {
  id_to_index: HashMap<EntityId, usize>,
  index_to_id: Vec<EntityId>,

  // TODO: Use yaglw's ArrayHandle.
  empty_array: GLuint,
  length: usize,

  // Per-triangle buffers

  vertex_positions: BufferTexture<'a, Triangle<Pnt3<GLfloat>>>,
  normals: BufferTexture<'a, Triangle<Vec3<GLfloat>>>,
  // Index into `pixels`.
  block_indices: BufferTexture<'a, GLuint>,
  // 2D coordinates into a texture in `pixels`.
  coords: BufferTexture<'a, Triangle<Pnt2<GLfloat>>>,

  // Per-block buffers

  block_to_index: HashMap<BlockPosition, GLuint>,
  free_list: Vec<GLuint>,
  lods: BufferTexture<'a, GLuint>,
  pixel_indices: BufferTexture<'a, GLuint>,

  pixels: [PixelBuffer<'a>; 4],
}

#[test]
fn correct_size() {
  assert!(mem::size_of::<Triangle<Pnt3<GLfloat>>>() == 3 * mem::size_of::<Pnt3<GLfloat>>());
  assert!(mem::size_of::<Pnt2<GLfloat>>() == 2 * mem::size_of::<GLfloat>());
  assert!(mem::size_of::<Pnt3<GLfloat>>() == 3 * mem::size_of::<GLfloat>());
  assert!(mem::size_of::<Vec3<GLfloat>>() == 3 * mem::size_of::<GLfloat>());
}

impl<'a> TerrainVRAMBuffers<'a> {
  pub fn new(
    gl: &'a GLContextExistence,
    gl_context: &mut GLContext,
  ) -> TerrainVRAMBuffers<'a> {
    let num_blocks = 65536;
    TerrainVRAMBuffers {
      id_to_index: HashMap::new(),
      index_to_id: Vec::new(),
      empty_array: unsafe {
        let mut empty_array = 0;
        gl::GenVertexArrays(1, &mut empty_array);
        empty_array
      },
      length: 0,
      vertex_positions: BufferTexture::new(gl, gl_context, gl::R32F, POLYGON_BUDGET),
      normals: BufferTexture::new(gl, gl_context, gl::R32F, POLYGON_BUDGET),
      coords: BufferTexture::new(gl, gl_context, gl::R32F, POLYGON_BUDGET),
      block_indices: BufferTexture::new(gl, gl_context, gl::R32UI, POLYGON_BUDGET),

      block_to_index: HashMap::new(),
      free_list: range(0, num_blocks as u32).collect(),
      lods: {
        let mut lods = BufferTexture::new(gl, gl_context, gl::R32UI, num_blocks);
        let init: Vec<_> = repeat(u32::MAX).take(num_blocks).collect();
        lods.buffer.push(gl_context, init.as_slice());
        lods
      },
      pixel_indices: {
        let mut pixels = BufferTexture::new(gl, gl_context, gl::R32UI, num_blocks);
        let init: Vec<_> = repeat(u32::MAX).take(num_blocks).collect();
        pixels.buffer.push(gl_context, init.as_slice());
        pixels
      },
      pixels: [
        PixelBuffer::new(gl, gl_context, texture_generator::TEXTURE_WIDTH[0], 32),
        PixelBuffer::new(gl, gl_context, texture_generator::TEXTURE_WIDTH[1], 2048),
        PixelBuffer::new(gl, gl_context, texture_generator::TEXTURE_WIDTH[2], 8192),
        PixelBuffer::new(gl, gl_context, texture_generator::TEXTURE_WIDTH[3], 32768),
      ],
    }
  }

  pub fn bind_glsl_uniforms(
    &self,
    gl: &mut GLContext,
    texture_unit_alloc: &mut IdAllocator<TextureUnit>,
    shader: &mut TerrainShader,
  ) {
    shader.shader.use_shader(gl);
    let mut bind = |&mut: name, id| {
      let unit = texture_unit_alloc.allocate();
      unsafe {
        gl::ActiveTexture(unit.gl_id());
        gl::BindTexture(gl::TEXTURE_BUFFER, id);
      }
      let loc = shader.shader.get_uniform_location(name);
      unsafe {
        gl::Uniform1i(loc, unit.glsl_id as GLint);
      }
    };

    bind("positions", self.vertex_positions.handle.gl_id);
    bind("normals", self.normals.handle.gl_id);
    bind("block_indices", self.block_indices.handle.gl_id);
    bind("coords", self.coords.handle.gl_id);

    bind("pixels_0", self.pixels[0].buffer.handle.gl_id);
    bind("pixels_1", self.pixels[1].buffer.handle.gl_id);
    bind("pixels_2", self.pixels[2].buffer.handle.gl_id);
    bind("pixels_3", self.pixels[3].buffer.handle.gl_id);

    bind("lods", self.lods.handle.gl_id);
    bind("pixel_indices", self.pixel_indices.handle.gl_id);
  }

  pub fn push(
    &mut self,
    gl: &mut GLContext,
    vertices: &[Triangle<Pnt3<GLfloat>>],
    normals: &[Triangle<Vec3<GLfloat>>],
    coords: &[Triangle<Pnt2<f32>>],
    block_indices: &[u32],
    ids: &[EntityId],
  ) -> bool {
    assert_eq!(vertices.len(), ids.len());
    assert_eq!(normals.len(), ids.len());
    assert_eq!(coords.len(), ids.len());
    assert_eq!(block_indices.len(), ids.len());

    self.vertex_positions.buffer.byte_buffer.bind(gl);
    let success = self.vertex_positions.buffer.push(gl, vertices);
    if !success {
      return false;
    }

    self.normals.buffer.byte_buffer.bind(gl);
    let success = self.normals.buffer.push(gl, normals);
    assert!(success);

    self.coords.buffer.byte_buffer.bind(gl);
    let success = self.coords.buffer.push(gl, coords);
    assert!(success);

    self.block_indices.buffer.byte_buffer.bind(gl);
    let success = self.block_indices.buffer.push(gl, block_indices);
    assert!(success);

    for &id in ids.iter() {
      self.id_to_index.insert(id, self.index_to_id.len());
      self.index_to_id.push(id);
    }

    self.length += VERTICES_PER_TRIANGLE as usize * ids.len();

    true
  }

  // TODO: Make this take many ids as a parameter, to reduce `bind`s.
  // Note: `id` must be present in the buffers.
  pub fn swap_remove(&mut self, gl: &mut GLContext, id: EntityId) {
    let idx = *self.id_to_index.get(&id).unwrap();
    let swapped_id = self.index_to_id[self.index_to_id.len() - 1];
    self.index_to_id.swap_remove(idx);
    self.id_to_index.remove(&id);

    if id != swapped_id {
      self.id_to_index.insert(swapped_id, idx);
    }

    self.length -= 3;

    self.vertex_positions.buffer.byte_buffer.bind(gl);
    self.vertex_positions.buffer.swap_remove(gl, idx, 1);

    self.normals.buffer.byte_buffer.bind(gl);
    self.normals.buffer.swap_remove(gl, idx, 1);

    self.coords.buffer.byte_buffer.bind(gl);
    self.coords.buffer.swap_remove(gl, idx, 1);

    self.block_indices.buffer.byte_buffer.bind(gl);
    self.block_indices.buffer.swap_remove(gl, idx, 1);
  }

  pub fn push_block_data(
    &mut self,
    gl: &mut GLContext,
    id: BlockPosition,
    pixels: &[Color3<GLfloat>],
    lod: u32,
  ) -> u32 {
    let block_idx;
    match self.free_list.pop() {
      None => panic!("Ran out of VRAM for block data."),
      Some(i) => block_idx = i,
    }
    self.lods.buffer.byte_buffer.bind(gl);
    self.lods.buffer.update(gl, block_idx as usize, &[lod]);

    let len = texture_generator::TEXTURE_LEN[lod as usize];
    assert_eq!(len, pixels.len());

    let pixel_idx;
    match self.pixels[lod as usize].push(gl, pixels, id) {
      None => panic!("Ran out of texture VRAM for LOD: {}.", lod),
      Some(i) => pixel_idx = i,
    };

    self.pixel_indices.buffer.byte_buffer.bind(gl);
    self.pixel_indices.buffer.update(gl, block_idx as usize, &[pixel_idx]);

    self.block_to_index.insert(id, block_idx);

    block_idx
  }

  pub fn free_block_data(&mut self, lod: u32, id: &BlockPosition) -> bool {
    match self.block_to_index.remove(id) {
      None => false,
      Some(idx) => {
        self.free_list.push(idx);
        let idx = self.pixels[lod as usize].block_to_index.remove(id).unwrap();
        self.pixels[lod as usize].free_list.push(idx);
        true
      }
    }
  }

  pub fn draw(&self, _gl: &mut GLContext) {
    unsafe {
      gl::BindVertexArray(self.empty_array);
      gl::DrawArrays(gl::TRIANGLES, 0, self.length as GLint);
    }
  }
}

struct PixelBuffer<'a> {
  buffer: BufferTexture<'a, Color3<GLfloat>>,
  // Map each block to a (2D) location in buffer.
  block_to_index: HashMap<BlockPosition, GLuint>,
  // List of free (2D) texture locations in `buffer`.
  free_list: Vec<GLuint>,
}

impl<'a> PixelBuffer<'a> {
  pub fn new(
    gl: &'a GLContextExistence,
    gl_context: &mut GLContext,
    texture_width: u32,
    len: u32,
  ) -> PixelBuffer<'a> {
    let tex_len = texture_width * texture_width;
    let buf_len = (tex_len * len) as usize;
    PixelBuffer {
      buffer: {
        let mut buffer = BufferTexture::new(gl, gl_context, gl::R32F, buf_len);
        let init = Color3::of_rgb(0.0, 0.0, 0.0);
        let init: Vec<_> = repeat(init).take(buf_len).collect();
        buffer.buffer.push(gl_context, init.as_slice());
        buffer
      },
      block_to_index: HashMap::new(),
      free_list: range(0, len).collect(),
    }
  }

  pub fn push(
    &mut self,
    gl: &mut GLContext,
    pixels: &[Color3<GLfloat>],
    id: BlockPosition,
  ) -> Option<u32> {
    self.free_list.pop().map(|idx| {
      self.buffer.buffer.byte_buffer.bind(gl);
      self.buffer.buffer.update(gl, idx as usize * pixels.len(), pixels);

      self.block_to_index.insert(id, idx);
      idx
    })
  }
}
