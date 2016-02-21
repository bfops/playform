//! Draw linearly-interpolated colored vertices in 3D space.

use gl;
use yaglw::gl_context::GLContext;
use yaglw::shader::Shader;

#[allow(missing_docs)]
pub struct ColorShader<'a> {
  #[allow(missing_docs)]
  pub shader: Shader<'a>,
}

impl<'a> ColorShader<'a> {
  #[allow(missing_docs)]
  pub fn new<'b>(gl: &'b GLContext) -> Self where 'a: 'b {
    let components = vec!(
      (gl::FRAGMENT_SHADER, "
        #version 330 core

        in vec4 color;
        out vec4 frag_color;

        void main() {
          frag_color = color;
        }".to_owned()),
      (gl::VERTEX_SHADER, "
        #version 330 core

        uniform mat4 projection_matrix;

        in vec3 position;
        in vec4 in_color;

        out vec4 color;

        void main() {
          gl_Position = projection_matrix * vec4(position, 1.0);
          color = in_color;
        }".to_owned()),
    );
    ColorShader {
      shader: Shader::new(gl, components.into_iter()),
    }
  }
}
