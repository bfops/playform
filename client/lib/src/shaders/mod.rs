//! This module contains the game's custom shader structs.

pub mod color;
pub mod grass_billboard;
pub mod terrain;
pub mod texture;

mod adjust_depth_precision;
mod depth_fog;
mod noise;
mod world_fragment;

mod bark;
mod sky;
mod dirt;
mod grass;
mod leaves;
mod stone;

use camera;
use cgmath;
use cgmath::{Vector2};
use gl;
use yaglw::gl_context::GLContext;

/// The game's custom shader structs.
pub struct Shaders<'a> {
  #[allow(missing_docs)]
  pub mob_shader: self::color::ColorShader<'a>,
  #[allow(missing_docs)]
  pub terrain_shader: self::terrain::TerrainShader<'a>,
  #[allow(missing_docs)]
  pub texture_shader: self::texture::TextureShader<'a>,
  #[allow(missing_docs)]
  pub grass_billboard: self::grass_billboard::T<'a>,
  #[allow(missing_docs)]
  pub hud_color_shader: self::color::ColorShader<'a>,
  #[allow(missing_docs)]
  pub sky: self::sky::T<'a>,
}

impl<'a> Shaders<'a> {
  #[allow(missing_docs)]
  pub fn new<'b>(gl: &'b mut GLContext, window_size: Vector2<i32>, near: f32, far: f32) -> Self where 'a: 'b {
    let terrain_shader = self::terrain::TerrainShader::new(gl, near, far);
    let mob_shader = self::color::ColorShader::new(gl, near, far);
    let mut hud_color_shader = self::color::ColorShader::new(gl, 0.0, 1.0);
    let texture_shader = self::texture::TextureShader::new(gl);
    let grass_billboard = self::grass_billboard::new(gl, near, far);
    let sky = self::sky::new(gl);

    let hud_camera = {
      let mut c = camera::Camera::unit();
      let dx = window_size.x as f32 / window_size.y as f32;
      let dy = 1.0;
      c.fov = cgmath::ortho(-dx, dx, -dy, dy, -1.0, 1.0);
      c
    };

    camera::set_camera(
      &mut hud_color_shader.shader,
      gl,
      &hud_camera,
    );

    match gl.get_error() {
      gl::NO_ERROR => {},
      err => warn!("OpenGL error 0x{:x}", err),
    }

    Shaders {
      mob_shader: mob_shader,
      terrain_shader: terrain_shader,
      texture_shader: texture_shader,
      grass_billboard: grass_billboard,
      hud_color_shader: hud_color_shader,
      sky: sky,
    }
  }
}
