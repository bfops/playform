use camera;
use camera::Camera;
use common::*;
use fontloader::FontLoader;
use gl;
use gl::types::*;
use id_allocator::IdAllocator;
use mob;
use nalgebra::Vec2;
use shaders::Shaders;
use std::f32::consts::PI;
use terrain::terrain_vram_buffers::TerrainVRAMBuffers;
use vertex::{ColoredVertex, TextureVertex};
use yaglw::gl_context::GLContext;
use yaglw::vertex_buffer::{GLArray, GLBuffer, GLType, DrawMode, VertexAttribData};
use yaglw::texture::{Texture2D, TextureUnit};

pub struct View<'a> {
  pub gl: GLContext,
  pub shaders: Shaders<'a>,

  pub terrain_buffers: TerrainVRAMBuffers<'a>,
  pub mob_buffers: mob::MobBuffers<'a>,
  pub hud_triangles: GLArray<'a, ColoredVertex>,
  pub text_triangles: GLArray<'a, TextureVertex>,

  pub misc_texture_unit: TextureUnit,
  pub text_textures: Vec<Texture2D<'a>>,
  pub fontloader: FontLoader,

  pub camera: Camera,
}

impl<'a> View<'a> {
  pub fn new(mut gl: GLContext, window_size: Vec2<i32>) -> View<'a> {
    let mut texture_unit_alloc = IdAllocator::new();

    let mut shaders = Shaders::new(&mut gl, window_size);

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

    unsafe {
      gl::FrontFace(gl::CCW);
      gl::CullFace(gl::BACK);
      gl::Enable(gl::CULL_FACE);

      gl::Enable(gl::BLEND);
      gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

      gl::Enable(gl::LINE_SMOOTH);
      gl::LineWidth(2.5);

      gl::Enable(gl::DEPTH_TEST);
      gl::DepthFunc(gl::LESS);
      gl::ClearDepth(1.0);
    }

    unsafe {
      gl::ActiveTexture(misc_texture_unit.gl_id());
    }

    let texture_in =
      shaders.hud_texture_shader.shader.get_uniform_location("texture_in");
    shaders.hud_texture_shader.shader.use_shader(&mut gl);
    unsafe {
      gl::Uniform1i(texture_in, misc_texture_unit.glsl_id as GLint);
    }

    View {
      gl: gl,
      shaders: shaders,

      terrain_buffers: terrain_buffers,
      mob_buffers: mob_buffers,
      hud_triangles: hud_triangles,
      text_triangles: text_triangles,

      misc_texture_unit: misc_texture_unit,
      text_textures: text_textures,
      fontloader: FontLoader::new(),

      camera: {
        let mut camera = Camera::unit();
        // Initialize the projection matrix.
        camera.fov = camera::perspective(3.14/3.0, 4.0/3.0, 0.1, 2048.0);
        camera.rotate_lateral(PI / 2.0);
        camera
      },
    }
  }
}
