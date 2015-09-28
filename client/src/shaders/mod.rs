//! This module contains the game's custom shader structs.

pub mod color;
pub mod deferred;
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
use yaglw::texture::TextureUnit;

use common::color::Color3;

/// The game's custom shader structs.
pub struct T<'a> {
  #[allow(missing_docs)]
  pub mob: self::color::T<'a>,
  #[allow(missing_docs)]
  pub terrain: self::terrain::T<'a>,
  #[allow(missing_docs)]
  pub hud_texture: self::texture::T<'a>,
  #[allow(missing_docs)]
  pub hud_color: self::color::T<'a>,
  #[allow(missing_docs)]
  pub deferred: self::deferred::T<'a>,
}

#[allow(missing_docs)]
pub fn new<'a, 'b:'a>(
  gl: &'a mut GLContext, 
  window_size: Vector2<i32>,
  positions: &TextureUnit,
  depth: &TextureUnit,
  normals: &TextureUnit,
  material: &TextureUnit,
) -> T<'b> {
  let terrain = self::terrain::new(gl);
  let mob = self::color::new(gl);
  let mut hud_color = self::color::new(gl);
  let mut hud_texture = self::texture::new(gl);
  let mut deferred = self::deferred::new(gl, positions, depth, normals, material);

  set_sun(
    &mut deferred.shader,
    gl,
    &light::Sun {
      direction: Vector3::new(0.0, 0.0, 0.0),
      intensity: Color3::of_rgb(0.0, 0.0, 0.0),
    }
  );
  set_ambient_light(
    &mut deferred.shader,
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
    &mut hud_color.shader,
    gl,
    &hud_camera,
  );
  camera::set_camera(
    &mut hud_texture.shader,
    gl,
    &hud_camera,
  );

  match gl.get_error() {
    gl::NO_ERROR => {},
    err => warn!("OpenGL error 0x{:x}", err),
  }

  T {
    mob: mob,
    terrain: terrain,
    hud_texture: hud_texture,
    hud_color: hud_color,
    deferred: deferred,
  }
}
