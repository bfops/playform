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
pub fn new<'a, 'b:'a>(gl: &'a GLContext, near: f32, far: f32) -> T<'b> {
  let components = vec!(
    (gl::VERTEX_SHADER, format!(r#"
      #version 330 core

      uniform mat4 projection_matrix;
      uniform vec3 eye_position;

      in vec3 vertex_position;
      in vec2 texture_position;
      in ivec3 tex_id;
      in vec4 model_matrix_col0;
      in vec4 model_matrix_col1;
      in vec4 model_matrix_col2;
      in vec4 model_matrix_col3;
      in vec3 normal;

      out vec2 vs_texture_position;
      out vec3 vs_normal;
      out float vs_tex_id;

      // include adjust_depth_precision
      {}

      mat3 between(vec3 v1, vec3 v2) {{
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
      }}

      void main() {{
        mat4 model_matrix = mat4(model_matrix_col0, model_matrix_col1, model_matrix_col2, model_matrix_col3);
        vs_texture_position = texture_position;
        vs_tex_id = float(tex_id[gl_VertexID / 6]);
        vs_normal = normal;
        vec4 root = model_matrix * vec4(0, 0, 0, 1);
        float scale = exp(-length(vec3(root / root.w) - eye_position) / 64);
        gl_Position =
          adjust_depth_precision(
            projection_matrix *
            model_matrix *
            vec4(scale * vertex_position, 1)
          );
      }}"#,
      ::shaders::adjust_depth_precision::as_string(near, far),
    )),
    (gl::FRAGMENT_SHADER, format!("
      #version 330 core

      uniform struct Sun {{
        vec3 direction;
        vec3 intensity;
      }} sun;

      uniform vec3 ambient_light;
      uniform vec3 eye_position;

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
        vec3 world_position = vec3(gl_FragCoord.xy * gl_FragCoord.w, gl_FragCoord.w);
        vec4 fog_color = vec4(sun.intensity, 1);
        frag_color =
          world_fragment(
            sun.direction,
            sun.intensity,
            normalize(world_position - eye_position),
            ambient_light,
            c,
            1.0 / 0.0,
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
