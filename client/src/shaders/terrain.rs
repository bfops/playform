//! Read and draw terrain data in 3D.

use common::terrain_block::{TEXTURE_WIDTH, TEXTURE_LEN};
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

        uniform struct Light {{
          vec3 position;
          vec3 intensity;
        }} light;

        uniform vec3 ambient_light;

        uniform samplerBuffer positions;

        flat in int face_id;
        in vec3 world_position;
        in vec3 normal;

        out vec4 frag_color;

        void main() {{
          int tex_width[4];
          int tex_length[4];
          tex_width[0] = {};
          tex_length[0] = {};
          tex_width[1] = {};
          tex_length[1] = {};
          tex_width[2] = {};
          tex_length[2] = {};
          tex_width[3] = {};
          tex_length[3] = {};

          int color_id = face_id * 3;

          vec4 base_color;

          base_color.r = 0.0;
          base_color.g = 1.0;
          base_color.b = 0.0;
          base_color.a = 1.0;

          // vector from here to the light
          vec3 light_path = light.position - world_position;
          light_path = normalize(light_path);
          float brightness = dot(normal, light_path);
          brightness = clamp(brightness, 0, 1);

          vec3 lighting = brightness * light.intensity + ambient_light;
          frag_color = vec4(clamp(lighting, 0, 1), 1) * base_color;
        }}",
          TEXTURE_WIDTH[0], TEXTURE_LEN[0],
          TEXTURE_WIDTH[1], TEXTURE_LEN[1],
          TEXTURE_WIDTH[2], TEXTURE_LEN[2],
          TEXTURE_WIDTH[3], TEXTURE_LEN[3],
        )),
    );
    TerrainShader {
      shader: Shader::new(gl, components.into_iter()),
    }
  }
}
