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
    (gl::FRAGMENT_SHADER,
      format!(r#"
        #version 330 core

        uniform vec2 window_size;

        uniform struct Sun {{
          vec3 direction;
          vec3 intensity;
        }} sun;

        const float sun_angular_radius = 3.14/32;

        uniform mat4 projection_matrix;
        uniform vec3 eye_position;

        uniform float time_ms;

        out vec4 frag_color;

        // include depth fog
        {}

        // include cnoise
        {}

        vec3 pixel_direction(vec2 pixel) {{
          // Scale to [0, 1]
          pixel /= window_size;
          // Scale to [-1, 1]
          pixel = 2*pixel - 1;
          vec4 p = inverse(projection_matrix) * vec4(pixel, -1, 1);
          return normalize(vec3(p / p.w) - eye_position);
        }}

        float cloud_noise(vec3 seed) {{
          float f = cnoise(seed + time_ms / 10000);
          // to [0, 1]
          f = f / 2 + 0.5;
          return f;
        }}

        void main() {{
          vec3 c = sun.intensity;

          vec3 direction = pixel_direction(gl_FragCoord.xy);

          const int HEIGHTS = 2;
          float heights[HEIGHTS] = float[](150, 1000);
          vec3 offsets[HEIGHTS] = vec3[](vec3(12,553,239), vec3(-10, 103, 10004));

          if (dot(normalize(sun.direction), direction) > cos(sun_angular_radius)) {{
            c = vec3(1);
          }}

          float alpha = 0;
          for (int i = 0; i < HEIGHTS; ++i) {{
            float cloud_height = heights[i];
            float dist = (cloud_height - eye_position.y) / direction.y;
            if (dist <= 0 || dist > 1000000) {{
              continue;
            }} else {{
              vec3 seed = (eye_position + dist * direction + offsets[i]) / 1000 * vec3(1, 4, 1);
              float f = cloud_noise(seed) * cloud_noise(seed + vec3(-10, -103, 1));
              f = f * f;
              alpha += f * (1 - fog_density(dist / 8));
            }}
          }}

          alpha = min(alpha, 1);
          c = mix(c, vec3(1, 1, 1), alpha);

          frag_color = vec4(c, 1);
        }}"#,
        ::shaders::depth_fog::to_string(),
        ::shaders::noise::cnoise(),
      )
    ),
  );
  T {
    shader: Shader::new(gl, components.into_iter()),
  }
}
