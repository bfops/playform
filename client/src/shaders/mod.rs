//! This module contains the game's custom shader structs.

pub mod color;
pub mod terrain;
pub mod texture;

mod noise;

mod bark;
mod dirt;
mod grass;
mod leaves;
mod stone;

use camera;
use cgmath;
use cgmath::{Vector2, Vector3};
use gl;
use light;
use light::{set_sun, set_ambient_light};
use yaglw::gl_context::GLContext;

use common::color::Color3;

/// The game's custom shader structs.
pub struct Shaders<'a> {
  #[allow(missing_docs)]
  pub mob_shader: self::color::ColorShader<'a>,
  #[allow(missing_docs)]
  pub terrain_shader: self::terrain::TerrainShader<'a>,
  #[allow(missing_docs)]
  pub hud_texture_shader: self::texture::TextureShader<'a>,
  #[allow(missing_docs)]
  pub hud_color_shader: self::color::ColorShader<'a>,
}

impl<'a> Shaders<'a> {
  #[allow(missing_docs)]
  pub fn new<'b:'a>(gl: &'a mut GLContext, window_size: Vector2<i32>) -> Shaders<'b> {
    let mut terrain_shader = self::terrain::TerrainShader::new(gl);
    let mob_shader = self::color::ColorShader::new(gl);
    let mut hud_color_shader = self::color::ColorShader::new(gl);
    let mut hud_texture_shader = self::texture::TextureShader::new(gl);

    set_sun(
      &mut terrain_shader.shader,
      gl,
      &light::Sun {
        direction: Vector3::new(0.0, 0.0, 0.0),
        intensity: Color3::of_rgb(0.0, 0.0, 0.0),
      }
    );
    set_ambient_light(
      &mut terrain_shader.shader,
      gl,
      Color3::of_rgb(0.4, 0.4, 0.4),
    );

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
    camera::set_camera(
      &mut hud_texture_shader.shader,
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
      hud_texture_shader: hud_texture_shader,
      hud_color_shader: hud_color_shader,
    }
  }
}
