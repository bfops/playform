//! This module contains the game's custom shader structs.

pub mod color;
pub mod terrain;
pub mod texture;

use camera;
use common::color::Color3;
use common::matrix;
use gl;
use light::{Light, set_point_light, set_ambient_light};
use nalgebra::{Pnt3, Vec2};
use yaglw::gl_context::GLContext;

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
  pub fn new<'b:'a>(gl: &'a mut GLContext, window_size: Vec2<i32>) -> Shaders<'b> {
    let mut terrain_shader = self::terrain::TerrainShader::new(gl);
    let mob_shader = self::color::ColorShader::new(gl);
    let mut hud_color_shader = self::color::ColorShader::new(gl);
    let mut hud_texture_shader = self::texture::TextureShader::new(gl);

    set_point_light(
      &mut terrain_shader.shader,
      gl,
      &Light {
        position: Pnt3::new(0.0, 0.0, 0.0),
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
      c.fov = matrix::sortho(window_size.x as f32 / window_size.y as f32, 1.0, -1.0, 1.0);
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
