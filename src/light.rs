use gl;
use gl::types::GLfloat;
use glw::gl_context::GLContext;
use glw::shader::Shader;
use nalgebra::Vec3;

pub struct Light {
  pub position: Vec3<GLfloat>,
  pub intensity: Vec3<GLfloat>,
}

/// Sets the variable `light` in some shader.
pub fn set_point_light(shader: &mut Shader, gl: &mut GLContext, light: &Light) {
  let light_position = shader.get_uniform_location("light.position");
  let light_intensity = shader.get_uniform_location("light.intensity");
  shader.use_shader(gl);
  unsafe {
    gl::Uniform3f(light_position, light.position.x, light.position.y, light.position.z);
    gl::Uniform3f(light_intensity, light.intensity.x, light.intensity.y, light.intensity.z);
  }
}

pub fn set_ambient_light(shader: &mut Shader, gl: &mut GLContext, intensity: Vec3<GLfloat>) {
  let ambient_light = shader.get_uniform_location("ambient_light");
  shader.use_shader(gl);
  unsafe {
    gl::Uniform3f(ambient_light, intensity.x, intensity.y, intensity.z);
  }
}
