use gl;
use yaglw::gl_context::GLContext;
use yaglw::shader::Shader;

pub struct ColorShader<'a> {
  pub shader: Shader<'a>,
}

impl<'a> ColorShader<'a> {
  pub fn new<'b:'a>(gl: &'a GLContext) -> ColorShader<'b> {
    let components = vec!(
      (gl::FRAGMENT_SHADER, "
        #version 330 core

        in vec4 color;
        out vec4 frag_color;

        void main() {
          frag_color = color;
        }".to_string()),
      (gl::VERTEX_SHADER, "
        #version 330 core

        uniform mat4 projection_matrix;

        in vec3 position;
        in vec4 in_color;

        out vec4 color;

        void main() {
          gl_Position = projection_matrix * vec4(position, 1.0);
          color = in_color;
        }".to_string()),
    );
    ColorShader {
      shader: Shader::new(gl, components.into_iter()),
    }
  }
}
