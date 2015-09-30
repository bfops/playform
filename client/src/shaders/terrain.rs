//! Read and draw terrain data in 3D.

use gl;
use yaglw::gl_context::GLContext;
use yaglw::shader::Shader;

#[allow(missing_docs)]
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

      uniform samplerBuffer positions;
      uniform samplerBuffer normals;
      uniform isamplerBuffer materials;

      out vec3 vs_world_position;
      out vec3 vs_normal;
      flat out int vs_material;

      void main() {
        // Mutiply by 3 because there are 3 components for each normal vector.
        int position_id = gl_VertexID * 3;
        vs_world_position.x = texelFetch(positions, position_id + 0).r;
        vs_world_position.y = texelFetch(positions, position_id + 1).r;
        vs_world_position.z = texelFetch(positions, position_id + 2).r;

        int normal_id = position_id;
        vs_normal.x = texelFetch(normals, normal_id + 0).r;
        vs_normal.y = texelFetch(normals, normal_id + 1).r;
        vs_normal.z = texelFetch(normals, normal_id + 2).r;

        int face_id = gl_VertexID / 3;

        vs_material = texelFetch(materials, face_id).r;

        gl_Position = projection_matrix * vec4(vs_world_position, 1.0);
      }".to_owned()),
    (gl::FRAGMENT_SHADER, format!("
      #version 330 core

      in vec3 vs_world_position;
      in vec3 vs_normal;
      flat in int vs_material;

      layout (location = 0) out vec3 normal;
      layout (location = 1) out vec3 world_position;
      layout (location = 2) out int material;

      void main() {{
        normal = vs_normal;
        world_position = vs_world_position;
        material = vs_material;
      }}",
    )),
  );
  T {
    shader: Shader::new(gl, components.into_iter()),
  }
}
