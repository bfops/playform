//! Data structures and functions for dealing with lighting.

use common::color::Color3;
use gl;
use gl::types::GLfloat;
use cgmath::Vector3;
use yaglw::gl_context::GLContext;
use yaglw::shader::Shader;

#[derive(Debug, Clone)]
/// Colored sun data structure.
pub struct Sun {
  #[allow(missing_docs)]
  /// Normalized vector to the sun
  pub direction: Vector3<GLfloat>,
  #[allow(missing_docs)]
  pub intensity: Color3<GLfloat>,
}

/// Sets the `sun` struct in some shader.
pub fn set_sun(shader: &mut Shader, gl: &mut GLContext, sun: &Sun) {
  let sun_direction = shader.get_uniform_location("sun.direction");
  let sun_intensity = shader.get_uniform_location("sun.intensity");
  shader.use_shader(gl);
  unsafe {
    gl::Uniform3f(sun_direction, sun.direction.x, sun.direction.y, sun.direction.z);
    gl::Uniform3f(sun_intensity, sun.intensity.r, sun.intensity.g, sun.intensity.b);
  }
}

/// Sets the variable `ambient_light` in some shader.
pub fn set_ambient_light(shader: &mut Shader, gl: &mut GLContext, intensity: Color3<GLfloat>) {
  let ambient_light = shader.get_uniform_location("ambient_light");
  shader.use_shader(gl);
  unsafe {
    gl::Uniform3f(ambient_light, intensity.r, intensity.g, intensity.b);
  }
}
