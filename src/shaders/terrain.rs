use gl;
use terrain_texture;
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
        uniform samplerBuffer coords;
        uniform samplerBuffer normals;

        flat out int vertex_id;
        out vec3 normal;
        out vec2 pixel_coords;

        void main() {
          // Mutiply by 3 because there are 3 components for each normal vector.
          int position_id = gl_VertexID * 3;
          vec3 world_position;
          world_position.x = texelFetch(positions, position_id + 0).r;
          world_position.y = texelFetch(positions, position_id + 1).r;
          world_position.z = texelFetch(positions, position_id + 2).r;

          int normal_id = position_id;
          normal.x = texelFetch(normals, normal_id + 0).r;
          normal.y = texelFetch(normals, normal_id + 1).r;
          normal.z = texelFetch(normals, normal_id + 2).r;

          // There are 2 coords per vertex.
          int coord_id = gl_VertexID * 2;
          pixel_coords.x = texelFetch(coords, coord_id + 0).r;
          pixel_coords.y = texelFetch(coords, coord_id + 1).r;

          vertex_id = gl_VertexID;

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
        uniform samplerBuffer pixels;
        uniform isamplerBuffer block_indices;

        flat in int vertex_id;
        in vec3 normal;
        in vec2 pixel_coords;

        out vec4 frag_color;

        void main() {{
          int tex_length = {};
          int tex_width = {};

          int face_id = vertex_id / 3;
          int color_id = face_id * 3;

          int block_index = texelFetch(block_indices, face_id).r;

          vec4 base_color;
          int p_x = int(pixel_coords.x);
          int p_y = int(pixel_coords.y);
          if (p_x >= tex_width) {{
            p_x = tex_width - 1;
          }}
          if (p_y >= tex_width) {{
            p_y = tex_width - 1;
          }}

          int pixel_idx = block_index*tex_length + p_y*tex_width + p_x;
          // There are 3 components for every color.
          pixel_idx = pixel_idx * 3;
          base_color.r = texelFetch(pixels, pixel_idx + 0).r;
          base_color.g = texelFetch(pixels, pixel_idx + 1).r;
          base_color.b = texelFetch(pixels, pixel_idx + 2).r;
          base_color.a = 1.0;

          // Mutiply by 3 because there are 3 components for each position vector.
          int position_id = vertex_id * 3;
          vec3 world_position;
          world_position.x = texelFetch(positions, position_id + 0).r;
          world_position.y = texelFetch(positions, position_id + 1).r;
          world_position.z = texelFetch(positions, position_id + 2).r;

          // vector from here to the light
          vec3 light_path = light.position - world_position;
          light_path = normalize(light_path);
          float brightness = dot(normal, light_path);
          brightness = clamp(brightness, 0, 1);

          vec3 lighting = brightness * light.intensity + ambient_light;
          frag_color = vec4(clamp(lighting, 0, 1), 1) * base_color;
        }}", terrain_texture::TEXTURE_LEN, terrain_texture::TEXTURE_WIDTH)),
    );
    TerrainShader {
      shader: Shader::new(gl, components.into_iter()),
    }
  }
}
