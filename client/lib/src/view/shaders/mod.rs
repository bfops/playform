//! This module contains the game's custom shader structs.

pub mod color;
pub mod grass_billboard;
pub mod sky;
pub mod terrain;
pub mod texture;

use cgmath;
use cgmath::{Vector2};
use gl;
use std;
use yaglw::gl_context::GLContext;
use yaglw;

use view::camera;

pub fn shader_from_prefix<'a, 'b:'a>(gl: &'a GLContext, prefix: &'static str) -> yaglw::shader::Shader<'b> {
  let read_preprocessed_shader =
    |name| {
      String::from_utf8(
        std::process::Command::new("m4")
        .arg(name)
        .current_dir(std::path::Path::new("shaders/"))
        .output()
        .unwrap()
        .stdout
      ).unwrap()
    };
  let vs = read_preprocessed_shader(format!("{}.vs.glsl", prefix));
  let fs = read_preprocessed_shader(format!("{}.fs.glsl", prefix));
  debug!("loaded {} vertex shader:", prefix);
  debug!("{}", vs);
  debug!("loaded {} fragment shader:", prefix);
  debug!("{}", fs);
  let components =
    vec!(
      (gl::VERTEX_SHADER, vs),
      (gl::FRAGMENT_SHADER, fs),
    );
  yaglw::shader::Shader::new(gl, components.into_iter())
}

/// The game's custom shader structs.
pub struct T<'a> {
  #[allow(missing_docs)]
  pub mob_shader: self::color::T<'a>,
  #[allow(missing_docs)]
  pub terrain_shader: self::terrain::T<'a>,
  #[allow(missing_docs)]
  pub texture_shader: self::texture::T<'a>,
  #[allow(missing_docs)]
  pub grass_billboard: self::grass_billboard::T<'a>,
  #[allow(missing_docs)]
  pub hud_color_shader: self::color::T<'a>,
  #[allow(missing_docs)]
  pub sky: self::sky::T<'a>,
}

#[allow(missing_docs)]
pub fn new<'a, 'b>(gl: &'b mut GLContext, window_size: Vector2<i32>) -> T<'a> where 'a: 'b {
  let terrain_shader       = self::terrain::new(gl);
  let mob_shader           = self::color::new(gl);
  let mut hud_color_shader = self::color::new(gl);
  let texture_shader       = self::texture::new(gl);
  let grass_billboard      = self::grass_billboard::new(gl);
  let sky                  = self::sky::new(gl);

  let hud_camera = {
    let mut c = camera::unit();
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

  T {
    mob_shader: mob_shader,
    terrain_shader: terrain_shader,
    texture_shader: texture_shader,
    grass_billboard: grass_billboard,
    hud_color_shader: hud_color_shader,
    sky: sky,
  }
}
