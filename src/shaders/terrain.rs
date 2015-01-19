use gl;
use yaglw::gl_context::GLContextExistence;
use yaglw::shader::Shader;

pub struct TerrainShader<'a> {
  pub shader: Shader<'a>,
}

impl<'a> TerrainShader<'a> {
  pub fn new(gl: &'a GLContextExistence) -> TerrainShader<'a> {
    let components = vec!(
      (gl::VERTEX_SHADER, "
        #version 330 core

        uniform mat4 projection_matrix;

        uniform samplerBuffer positions;
        uniform samplerBuffer normals;

        flat out int vertex_id;
        out vec3 normal;

        void main() {
          // Mutiply by 3 because there are 3 components for each normal vector.
          int position_id = gl_VertexID * 3;
          vec3 world_position;
          world_position.x = texelFetch(positions, position_id).r;
          world_position.y = texelFetch(positions, position_id + 1).r;
          world_position.z = texelFetch(positions, position_id + 2).r;

          int normal_id = position_id;
          normal.x = texelFetch(normals, normal_id).r;
          normal.y = texelFetch(normals, normal_id + 1).r;
          normal.z = texelFetch(normals, normal_id + 2).r;

          vertex_id = gl_VertexID;

          gl_Position = projection_matrix * vec4(world_position, 1.0);
        }".to_string()),
      (gl::FRAGMENT_SHADER, "
        #version 330 core

        uniform struct Light {
          vec3 position;
          vec3 intensity;
        } light;

        uniform vec3 ambient_light;

        uniform samplerBuffer positions;
        uniform samplerBuffer colors;

        flat in int vertex_id;
        in vec3 normal;

        out vec4 frag_color;

        void main() {
          int face_id = vertex_id / 3;
          int color_id = face_id * 3;

          vec4 base_color;
          base_color.x = texelFetch(colors, color_id).r;
          base_color.y = texelFetch(colors, color_id + 1).r;
          base_color.z = texelFetch(colors, color_id + 2).r;
          base_color.w = 1.0;

          // Mutiply by 3 because there are 3 components for each position vector.
          int position_id = vertex_id * 3;
          vec3 world_position;
          world_position.x = texelFetch(positions, position_id).r;
          world_position.y = texelFetch(positions, position_id + 1).r;
          world_position.z = texelFetch(positions, position_id + 2).r;

          // vector from here to the light
          vec3 light_path = light.position - world_position;
          light_path = normalize(light_path);
          // length(normal) = 1 already.
          float brightness = dot(normal, light_path);
          brightness = clamp(brightness, 0, 1);

          vec3 lighting = brightness * light.intensity + ambient_light;
          frag_color = vec4(clamp(lighting, 0, 1), 1) * base_color;
        }".to_string()),
    );
    TerrainShader {
      shader: Shader::new(gl, components.into_iter()),
    }
  }
}
