use gl;
use yaglw::gl_context::GLContext;
use yaglw::shader::Shader;

pub struct T<'a> {
  #[allow(missing_docs)]
  pub shader: Shader<'a>,
}

pub fn new<'a, 'b:'a>(gl: &'a GLContext) -> T<'b> {
  let components = vec!(
    (gl::VERTEX_SHADER, "
      #version 330 core

      void main() {
        if (gl_VertexID == 0) {
          gl_Position = vec4(1, -1, 0, 1);
        } else if (gl_VertexID == 1) {
          gl_Position = vec4(1, 1, 0, 1);
        } else if (gl_VertexID == 2) {
          gl_Position = vec4(-1, -1, 0, 1);
        } else if (gl_VertexID == 3) {
          gl_Position = vec4(-1, 1, 0, 1);
        }
      }".to_owned()),
    (gl::FRAGMENT_SHADER, "
      #version 330 core

      uniform vec3 sun_color;

      out vec4 frag_color;

      void main() {
        frag_color = vec4(sun_color, 1);
      }".to_owned()),
  );
  T {
    shader: Shader::new(gl, components.into_iter()),
  }
}
