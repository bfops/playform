//! Read and draw terrain data in 3D.

use gl;
use yaglw::gl_context::GLContext;
use yaglw::shader::Shader;

#[allow(missing_docs)]
pub struct TerrainShader<'a> {
  #[allow(missing_docs)]
  pub shader: Shader<'a>,
}

impl<'a> TerrainShader<'a> {
  #[allow(missing_docs)]
  pub fn new<'b:'a>(gl: &'a GLContext) -> TerrainShader<'b> {
    let components = vec!(
      (gl::VERTEX_SHADER, "
        #version 330 core

        uniform mat4 projection_matrix;

        uniform samplerBuffer positions;
        uniform samplerBuffer normals;

        flat out int face_id;
        out vec3 world_position;
        out vec3 normal;

        void main() {
          // Mutiply by 3 because there are 3 components for each normal vector.
          int position_id = gl_VertexID * 3;
          world_position.x = texelFetch(positions, position_id + 0).r;
          world_position.y = texelFetch(positions, position_id + 1).r;
          world_position.z = texelFetch(positions, position_id + 2).r;

          int normal_id = position_id;
          normal.x = texelFetch(normals, normal_id + 0).r;
          normal.y = texelFetch(normals, normal_id + 1).r;
          normal.z = texelFetch(normals, normal_id + 2).r;

          face_id = gl_VertexID / 3;

          gl_Position = projection_matrix * vec4(world_position, 1.0);
        }".to_string()),
      (gl::FRAGMENT_SHADER, format!("
        #version 330 core

        uniform struct Sun {{
          vec3 direction;
          vec3 intensity;
        }} sun;

        uniform vec3 ambient_light;

        uniform samplerBuffer positions;

        flat in int face_id;
        in vec3 world_position;
        in vec3 normal;

        out vec4 frag_color;

        float perlin(const float x, const float y) {{
          float amplitude = 1;
          float frequency = 1.0 / 64.0;
          float persistence = 0.8;
          const float lacunarity = 2.4;
          const int octaves = 2;

          float r = 0.0;
          float max_crest = 0.0;

          for (int o = 0; o < octaves; ++o) {{
            float f = frequency * 2 * 3.14;
            r += amplitude * (sin((o+x) * f) + sin((o+y) * f)) / 2;

            max_crest += amplitude;
            amplitude *= persistence;
            frequency *= lacunarity;
          }}

          // Scale to [-1, 1]. N.B. There is no clamping.
          r /= max_crest;

          return r;
        }}

        void main() {{
          int color_id = face_id * 3;

          vec4 base_color = vec4(0, 0.5, 0, 1);

          float p = 0
            + perlin(0, world_position.x)
            + perlin(0, world_position.y)
            + perlin(0, world_position.z)
          ;

          // shift, scale, clamp to [0, 1]
          p = (p + 3) / 6;
          p = clamp(p, 0, 1);

          base_color.r = (1 - p) / 2;
          base_color.g = (3/2 + p) / 5;
          base_color.b = (1 - p) / 5;
          base_color.a = 1.0;

          float brightness = dot(normal, sun.direction);
          brightness = clamp(brightness, 0, 1);

          vec3 lighting = brightness * sun.intensity + ambient_light;
          frag_color = vec4(clamp(lighting, 0, 1), 1) * base_color;
        }}",
        )),
    );
    TerrainShader {
      shader: Shader::new(gl, components.into_iter()),
    }
  }
}
