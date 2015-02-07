use common::*;
use id_allocator::IdAllocator;
use mob;
use shaders::Shaders;
use terrain::terrain_vram_buffers::TerrainVRAMBuffers;
use yaglw::gl_context::GLContext;
use yaglw::vertex_buffer::{GLArray, GLBuffer, GLType, DrawMode, VertexAttribData};
use yaglw::texture::{Texture2D, TextureUnit};
use vertex::{ColoredVertex, TextureVertex};

pub struct Renderer<'a> {
  pub gl: GLContext,
  pub shaders: Shaders<'a>,

  pub terrain_buffers: TerrainVRAMBuffers<'a>,
  pub mob_buffers: mob::MobBuffers<'a>,
  pub hud_triangles: GLArray<'a, ColoredVertex>,
  pub text_triangles: GLArray<'a, TextureVertex>,

  pub misc_texture_unit: TextureUnit,
  pub text_textures: Vec<Texture2D<'a>>,
}

impl<'a> Renderer<'a> {
  pub fn new(mut gl: GLContext) -> Renderer<'a> {
    let mut texture_unit_alloc = IdAllocator::new();

    let mut shaders = Shaders::new(&mut gl);

    let terrain_buffers = TerrainVRAMBuffers::new(&mut gl);
    terrain_buffers.bind_glsl_uniforms(
      &mut gl,
      &mut texture_unit_alloc,
      &mut shaders.terrain_shader,
    );

    let mob_buffers = mob::MobBuffers::new(&mut gl, &shaders.mob_shader);

    let buffer = GLBuffer::new(&mut gl, 16 * VERTICES_PER_TRIANGLE as usize);
    let hud_triangles = {
      GLArray::new(
        &mut gl,
        &shaders.hud_color_shader.shader,
        &[
          VertexAttribData { name: "position", size: 3, unit: GLType::Float },
          VertexAttribData { name: "in_color", size: 4, unit: GLType::Float },
        ],
        DrawMode::Triangles,
        buffer,
      )
    };

    let buffer = GLBuffer::new(&mut gl, 8 * VERTICES_PER_TRIANGLE as usize);
    let text_triangles =
      GLArray::new(
        &mut gl,
        &shaders.hud_texture_shader.shader,
        &[
          VertexAttribData { name: "position", size: 3, unit: GLType::Float },
          VertexAttribData { name: "texture_position", size: 2, unit: GLType::Float },
        ],
        DrawMode::Triangles,
        buffer,
      );

    let text_textures = Vec::new();

    let misc_texture_unit = texture_unit_alloc.allocate();

    Renderer {
      gl: gl,
      shaders: shaders,

      terrain_buffers: terrain_buffers,
      mob_buffers: mob_buffers,
      hud_triangles: hud_triangles,
      text_triangles: text_triangles,

      misc_texture_unit: misc_texture_unit,
      text_textures: text_textures,
    }
  }
}
