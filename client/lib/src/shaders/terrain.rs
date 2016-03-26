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
  pub fn new<'b>(gl: &'b GLContext, near: f32, far: f32) -> Self where 'a: 'b {
    let components = vec!(
      (gl::VERTEX_SHADER, format!(r#"
        #version 330 core

        uniform mat4 projection_matrix;

        uniform samplerBuffer positions;
        uniform samplerBuffer normals;
        uniform isamplerBuffer materials;

        out vec3 world_position;
        out vec3 vs_normal;
        flat out int material;

        // include adjust_depth_precision
        {}

        void main() {{
          // Mutiply by 3 because there are 3 components for each normal vector.
          int position_id = gl_VertexID * 3;
          world_position.x = texelFetch(positions, position_id + 0).r;
          world_position.y = texelFetch(positions, position_id + 1).r;
          world_position.z = texelFetch(positions, position_id + 2).r;

          int normal_id = position_id;
          vs_normal.x = texelFetch(normals, normal_id + 0).r;
          vs_normal.y = texelFetch(normals, normal_id + 1).r;
          vs_normal.z = texelFetch(normals, normal_id + 2).r;

          int face_id = gl_VertexID / 3;

          material = texelFetch(materials, face_id).r;

          gl_Position = adjust_depth_precision(projection_matrix * vec4(world_position, 1.0));
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

        uniform samplerBuffer positions;

        in vec3 world_position;
        in vec3 vs_normal;
        flat in int material;

        out vec4 frag_color;

        // depth fog
        {}

        // world fragment shading
        {}

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

        // http://www.neilmendoza.com/glsl-rotation-about-an-arbitrary-axis/
        mat3 rotationMatrix(vec3 axis, float angle)
        {{
            axis = normalize(axis);
            float s = sin(angle);
            float c = cos(angle);
            float oc = 1.0 - c;

            return mat3(oc * axis.x * axis.x + c,           oc * axis.x * axis.y - axis.z * s,  oc * axis.z * axis.x + axis.y * s,
                        oc * axis.x * axis.y + axis.z * s,  oc * axis.y * axis.y + c,           oc * axis.y * axis.z - axis.x * s,
                        oc * axis.z * axis.x - axis.y * s,  oc * axis.y * axis.z + axis.x * s,  oc * axis.z * axis.z + c         );
        }}

        vec3 bump_map(float shallowness, float frequency, vec3 v) {{
          vec3 seed = frequency * world_position + vec3(0x123411);
          float p0 = cnoise(seed);
          float d = 0.1;
          float px = cnoise(seed + vec3(d, 0, 0));
          float py = cnoise(seed + vec3(0, d, 0));
          float pz = cnoise(seed + vec3(0, 0, d));
          vec3 r = normalize(vec3(px, py, pz) - vec3(p0));

          vec3 axis = cross(vec3(0, 1, 0), r);
          float c = dot(vec3(0, 1, 0), r);
          return rotationMatrix(axis, acos(c) / shallowness) * v;
        }}

        void main() {{
          vec4 base_color;

          vec3 normal = vs_normal;
          float shininess = 100000000;

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
            normal = bump_map(4, 2, normal);
          }} else if (material == 5) {{
            base_color = vec4(0, 0, 0, 1);
            shininess = 40;
          }} else {{
            base_color = vec4(0.5, 0, 0.5, 0.5);
            shininess = 1;
          }}

          vec4 fog_color = vec4(sun.intensity, 1);
          frag_color =
            world_fragment(
              sun.direction,
              sun.intensity,
              normalize(world_position - eye_position),
              ambient_light,
              base_color,
              shininess,
              normal,
              fog_color,
              gl_FragCoord.z / gl_FragCoord.w
            );
        }}",
        ::shaders::depth_fog::to_string(),
        ::shaders::world_fragment::to_string(),
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
