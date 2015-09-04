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

        // include cnoise
        {}

        vec4 grass() {{
          {}
        }}

        void main() {{
          int color_id = face_id * 3;

          vec4 base_color;

          base_color = grass();

          float brightness = dot(normal, sun.direction);
          brightness = clamp(brightness, 0, 1);
          vec3 lighting = brightness * sun.intensity + ambient_light;

          float fog_factor = gl_FragCoord.z / gl_FragCoord.w / 768;
          float fog_density = 1 - exp(-fog_factor);
          vec3 fog_color = sun.intensity;
          vec4 fog_component = vec4(fog_color, 1) * fog_density;
          vec4 material_component =
            vec4(clamp(lighting, 0, 1), 1) * base_color * vec4(1, 1, 1, 1 - fog_density);

          frag_color = fog_component + material_component;
        }}", ::shaders::noise::cnoise(), ::shaders::grass::grass(),
        )),
    );
    TerrainShader {
      shader: Shader::new(gl, components.into_iter()),
    }
  }
}
