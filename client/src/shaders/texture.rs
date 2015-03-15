//! Draw textures using a projection matrix.

use gl;
use yaglw::gl_context::GLContext;
use yaglw::shader::Shader;

/// Draw textures using a projection matrix.
pub struct TextureShader<'a> {
  #[allow(missing_docs)]
  pub shader: Shader<'a>,
}

impl<'a> TextureShader<'a> {
  #[allow(missing_docs)]
  pub fn new<'b:'a>(gl: &'a GLContext) -> TextureShader<'b> {
    let components = vec!(
      (gl::VERTEX_SHADER, "
        #version 330 core

        uniform mat4 projection_matrix;

        in vec3 position;
        in vec2 texture_position;

        out vec2 tex_position;

        void main() {
          tex_position = texture_position;
          gl_Position = projection_matrix * vec4(position, 1.0);
        }".to_string()),
      (gl::FRAGMENT_SHADER, "
        #version 330 core

        uniform sampler2D texture_in;

        in vec2 tex_position;

        out vec4 frag_color;

        void main() {
          frag_color = texture(texture_in, vec2(tex_position.x, 1.0 - tex_position.y));
        }".to_string()),
    );
    TextureShader {
      shader: Shader::new(gl, components.into_iter()),
    }
  }
}
