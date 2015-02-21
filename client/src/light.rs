//! Data structures and functions for dealing with lighting.

use common::color::Color3;
use gl;
use gl::types::GLfloat;
use cgmath::Point3;
use yaglw::gl_context::GLContext;
use yaglw::shader::Shader;

#[derive(Debug, Clone)]
/// Colored light data structure.
pub struct Light {
  #[allow(missing_docs)]
  pub position: Point3<GLfloat>,
  #[allow(missing_docs)]
  pub intensity: Color3<GLfloat>,
}

/// Sets the variable `light` in some shader.
pub fn set_point_light(shader: &mut Shader, gl: &mut GLContext, light: &Light) {
  let light_position = shader.get_uniform_location("light.position");
  let light_intensity = shader.get_uniform_location("light.intensity");
  shader.use_shader(gl);
  unsafe {
    gl::Uniform3f(light_position, light.position.x, light.position.y, light.position.z);
    gl::Uniform3f(light_intensity, light.intensity.r, light.intensity.g, light.intensity.b);
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
