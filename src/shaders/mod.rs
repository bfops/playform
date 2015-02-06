pub mod color;
pub mod terrain;
pub mod texture;

use camera;
use color::Color3;
use common::*;
use light::{Light, set_point_light, set_ambient_light};
use nalgebra::{Vec3, Pnt3};
use yaglw::gl_context::{GLContext, GLContextExistence};

pub struct Shaders<'a> {
  pub mob_shader: self::color::ColorShader<'a>,
  pub terrain_shader: self::terrain::TerrainShader<'a>,
  pub hud_texture_shader: self::texture::TextureShader<'a>,
  pub hud_color_shader: self::color::ColorShader<'a>,
}

impl<'a> Shaders<'a> {
  pub fn new(gl: &'a GLContextExistence, gl_context: &mut GLContext) -> Shaders<'a> {
    let mut terrain_shader = self::terrain::TerrainShader::new(gl);
    let mob_shader = self::color::ColorShader::new(gl);
    let mut hud_color_shader = self::color::ColorShader::new(gl);
    let mut hud_texture_shader = self::texture::TextureShader::new(gl);

    set_point_light(
      &mut terrain_shader.shader,
      gl_context,
      &Light {
        position: Pnt3::new(0.0, 0.0, 0.0),
        intensity: Color3::of_rgb(0.0, 0.0, 0.0),
      }
    );
    set_ambient_light(
      &mut terrain_shader.shader,
      gl_context,
      Color3::of_rgb(0.4, 0.4, 0.4),
    );

    let hud_camera = {
      let mut c = camera::Camera::unit();
      c.fov = camera::sortho(WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32, 1.0, -1.0, 1.0);
      c.fov = camera::translation(Vec3::new(0.0, 0.0, -1.0)) * c.fov;
      c
    };

    camera::set_camera(
      &mut hud_color_shader.shader,
      gl_context,
      &hud_camera,
    );
    camera::set_camera(
      &mut hud_texture_shader.shader,
      gl_context,
      &hud_camera,
    );

    Shaders {
      mob_shader: mob_shader,
      terrain_shader: terrain_shader,
      hud_texture_shader: hud_texture_shader,
      hud_color_shader: hud_color_shader,
    }
  }
}
