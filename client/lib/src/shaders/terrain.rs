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
        uniform isamplerBuffer materials;

        out vec3 world_position;
        out vec3 normal;
        flat out int material;

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

          int face_id = gl_VertexID / 3;

          material = texelFetch(materials, face_id).r;

          gl_Position = projection_matrix * vec4(world_position, 1.0);
        }".to_owned()),
      (gl::FRAGMENT_SHADER, format!("
        #version 330 core

        uniform struct Sun {{
          vec3 direction;
          vec3 intensity;
        }} sun;

        uniform vec3 ambient_light;

        uniform samplerBuffer positions;

        in vec3 world_position;
        in vec3 normal;
        flat in int material;

        out vec4 frag_color;

        // include cnoise
        {}

        vec3 grass() {{
          {}
        }}

        vec3 dirt() {{
          {}
        }}

        vec3 bark() {{
          {}
        }}

        vec3 leaves() {{
          {}
        }}

        vec3 stone() {{
          {}
        }}

        void main() {{
          vec4 base_color;

          if (material == 1) {{
            float grass_amount =
              (cnoise(world_position / 32) + 1) / 2 *
              dot(normal, vec3(0, 1, 0)) *
              1.5;
            grass_amount = clamp(grass_amount, 0, 1);
            base_color = vec4(mix(dirt(), grass(), grass_amount), 1);
          }} else if (material == 2) {{
            base_color = vec4(bark(), 1);
          }} else if (material == 3) {{
            base_color = vec4(leaves(), 1);
          }} else if (material == 4) {{
            base_color = vec4(stone(), 1);
          }} else {{
            base_color = vec4(0.5, 0, 0.5, 0.5);
          }}

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
        }}",
        ::shaders::noise::cnoise(),
        ::shaders::grass::grass(),
        ::shaders::dirt::dirt(),
        ::shaders::bark::bark(),
        ::shaders::leaves::leaves(),
        ::shaders::stone::stone(),
      )),
    );
    TerrainShader {
      shader: Shader::new(gl, components.into_iter()),
    }
  }
}
