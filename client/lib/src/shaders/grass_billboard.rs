//! Draw textures using a projection matrix.

use gl;
use yaglw::gl_context::GLContext;
use yaglw::shader::Shader;

/// Draw textures using a projection matrix.
pub struct T<'a> {
  #[allow(missing_docs)]
  pub shader: Shader<'a>,
}

#[allow(missing_docs)]
pub fn new<'a, 'b:'a>(gl: &'a GLContext) -> T<'b> {
  let components = vec!(
    (gl::VERTEX_SHADER, "
      #version 330 core

      uniform mat4 projection_matrix;

      in vec3 vertex_position;
      in vec2 texture_position;
      in vec3 root;
      in vec3 normal;
      in ivec3 tex_id;

      out vec2 vs_texture_position;
      out float vs_tex_id;

      mat3 between(vec3 v1, vec3 v2) {
        vec3 v = cross(v1, v2);
        float s = length(v);
        float c = dot(v1, v2);
        mat3 skew =
          mat3(
            vec3(0, v.z, -v.y),
            vec3(-v.z, 0, v.x),
            vec3(v.y, -v.x, 0)
          );
        return mat3(1) + skew + skew*skew*(1-c)/(s*s);
      }

      void main() {
        mat3 rot = between(vec3(0, 1, 0), normalize(normal));
        vs_texture_position = texture_position;
        vs_tex_id = float(tex_id[gl_VertexID / 6]);
        gl_Position = projection_matrix * vec4(root + rot*vertex_position, 1.0);
      }".to_owned()),
    (gl::FRAGMENT_SHADER, "
      #version 330 core

      uniform sampler2D texture_in;
      uniform float alpha_threshold;

      in vec2 vs_texture_position;
      in float vs_tex_id;

      out vec4 frag_color;

      void main() {
        int tex_id = int(round(vs_tex_id));
        int y = tex_id / 3;
        int x = tex_id % 3;
        vec2 tex_position = (vs_texture_position + y*vec2(0, 1) + x*vec2(1, 0)) / 3;
        vec4 c = texture(texture_in, tex_position);
        if (c.a < alpha_threshold) {
          discard;
        }
        frag_color = c;
      }".to_owned()),
  );
  T {
    shader: Shader::new(gl, components.into_iter()),
  }
}
