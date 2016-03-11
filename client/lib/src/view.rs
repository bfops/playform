//! The state associated with perceiving the world state.

use cgmath;
use gl;
use gl::types::*;
use std;
use yaglw::gl_context::GLContext;
use yaglw;
use yaglw::vertex_buffer::{GLArray, GLBuffer, GLType, DrawMode, VertexAttribData};
use yaglw::texture::{TextureUnit};

use camera::Camera;
use common;
use common::id_allocator;
use light;
use mob_buffers::MobBuffers;
use player_buffers::PlayerBuffers;
use shaders::Shaders;
use terrain_buffers::TerrainBuffers;
use vertex::{ColoredVertex};

const VERTICES_PER_TRIANGLE: usize = 3;

/// The state associated with perceiving the world state.
pub struct T<'a> {
  /// Current OpengL context.
  pub gl: GLContext,
  #[allow(missing_docs)]
  pub shaders: Shaders<'a>,

  #[allow(missing_docs)]
  pub terrain_buffers: TerrainBuffers<'a>,
  #[allow(missing_docs)]
  pub mob_buffers: MobBuffers<'a>,
  #[allow(missing_docs)]
  pub player_buffers: PlayerBuffers<'a>,
  /// Hud triangles for non-text.
  pub hud_triangles: GLArray<'a, ColoredVertex>,

  pub empty_gl_array: yaglw::vertex_buffer::ArrayHandle<'a>,

  /// A texture unit for misc use.
  pub misc_texture_unit: TextureUnit,

  #[allow(missing_docs)]
  pub sun: light::Sun,

  #[allow(missing_docs)]
  pub camera: Camera,

  #[allow(missing_docs)]
  pub show_hud: bool,
}

impl<'a> T<'a> {
  #[allow(missing_docs)]
  pub fn new(mut gl: GLContext, window_size: cgmath::Vector2<i32>) -> T<'a> {
    let mut texture_unit_alloc = id_allocator::new();

    let mut shaders = Shaders::new(&mut gl, window_size);

    let terrain_buffers = TerrainBuffers::new(&mut gl);
    terrain_buffers.bind_glsl_uniforms(
      &mut gl,
      &mut texture_unit_alloc,
      &mut shaders.terrain_shader,
    );

    let mob_buffers = MobBuffers::new(&mut gl, &shaders.mob_shader);
    let player_buffers = PlayerBuffers::new(&mut gl, &shaders.mob_shader);

    let buffer = GLBuffer::new(&mut gl, 16 * VERTICES_PER_TRIANGLE);
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

    let empty_gl_array = yaglw::vertex_buffer::ArrayHandle::new(&gl);

    T {
      gl: gl,
      shaders: shaders,

      terrain_buffers: terrain_buffers,
      mob_buffers: mob_buffers,
      player_buffers: player_buffers,
      hud_triangles: hud_triangles,

      empty_gl_array: empty_gl_array,

      misc_texture_unit: misc_texture_unit,

      camera: {
        let fovy = cgmath::rad(std::f32::consts::PI / 3.0);
        let aspect = window_size.x as f32 / window_size.y as f32;
        let mut camera = Camera::unit();
        // Initialize the projection matrix.
        camera.fov = cgmath::perspective(fovy, aspect, 0.1, 2048.0);
        // TODO: This should use player rotation from the server.
        camera.rotate_lateral(std::f32::consts::PI / 2.0);
        camera
      },

      sun:
        light::Sun {
          direction: cgmath::Vector3::new(0.0, 0.0, 0.0),
          intensity: common::color::Color3 { r: 0.0, g: 0.0, b: 0.0 },
        },

      show_hud: true,
    }
  }
}
