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
      out vec3 vs_normal;
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
        vs_normal = normal;
        gl_Position = projection_matrix * vec4(root + rot*vertex_position, 1.0);
      }".to_owned()),
    (gl::FRAGMENT_SHADER, format!("
      #version 330 core

      uniform struct Sun {{
        vec3 direction;
        vec3 intensity;
      }} sun;

      uniform vec3 ambient_light;

      uniform sampler2D texture_in;
      uniform float alpha_threshold;

      in vec2 vs_texture_position;
      in vec3 vs_normal;
      in float vs_tex_id;

      out vec4 frag_color;

      // depth fog
      {}

      // world fragment shading
      {}

      void main() {{
        int tex_id = int(round(vs_tex_id));
        int y = tex_id / 3;
        int x = tex_id % 3;
        vec2 tex_position = (vs_texture_position + y*vec2(0, 1) + x*vec2(1, 0)) / 3;
        vec4 c = texture(texture_in, tex_position);
        if (c.a < alpha_threshold) {{
          discard;
        }}
        vec4 fog_color = vec4(sun.intensity, 1);
        frag_color =
          world_fragment(
            sun.direction,
            sun.intensity,
            ambient_light,
            c,
            vs_normal,
            fog_color,
            gl_FragCoord.z / gl_FragCoord.w
          );
      }}",
      ::shaders::depth_fog::to_string(),
      ::shaders::world_fragment::to_string(),
    )),
  );
  T {
    shader: Shader::new(gl, components.into_iter()),
  }
}
