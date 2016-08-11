//! Draw linearly-interpolated colored vertices in 3D space.

use yaglw::gl_context::GLContext;
use yaglw::shader::Shader;

use view::shaders;

#[allow(missing_docs)]
pub struct T<'a> {
  #[allow(missing_docs)]
  pub shader: Shader<'a>,
}

#[allow(missing_docs)]
pub fn new<'a, 'b>(gl: &'b GLContext) -> T<'a> where 'a: 'b {
  T {
    shader: shaders::shader_from_prefix(gl, "grass_billboard")
  }
}
