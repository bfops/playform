//! Data structures and functions for dealing with lighting.

use cgmath::Vector3;
use gl;
use std;
use yaglw::gl_context::GLContext;
use yaglw::shader::Shader;

#[derive(Debug, Clone)]
/// Colored sun data structure.
pub struct Sun {
  /// How far through the day the sun is, in [0, 1).
  pub progression : f32,
  /// The rotation of the sun's path about the y axis.
  pub rotation: f32,
}

impl Sun {
  fn sin_cos(&self) -> (f32, f32) {
    // Convert to radians.
    let angle = self.progression * 2.0 * std::f32::consts::PI;
    angle.sin_cos()
  }

  pub fn direction(&self) -> Vector3<f32> {
    let (s, c) = self.sin_cos();
    Vector3::new(c, s, 0.0)
  }
}

/// Sets the `sun` struct in some shader.
pub fn set_sun(shader: &mut Shader, gl: &mut GLContext, sun: &Sun) {
  let sun_direction_uniform = shader.get_uniform_location("sun.direction");
  let sun_angular_radius_uniform = shader.get_uniform_location("sun.angular_radius");
  shader.use_shader(gl);
  unsafe {
    let d = sun.direction();
    gl::Uniform3f(sun_direction_uniform, d.x, d.y, d.z);
    gl::Uniform1f(sun_angular_radius_uniform, std::f32::consts::PI/32.0);
  }
}
