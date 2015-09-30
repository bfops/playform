//! Deferred shader.

use gl;
use yaglw::gl_context::GLContext;
use yaglw::shader::Shader;
use yaglw::texture::TextureUnit;

use view;

#[allow(missing_docs)]
pub struct T<'a> {
  #[allow(missing_docs)]
  pub shader: Shader<'a>,
}

#[allow(missing_docs)]
pub fn new<'a, 'b:'a>(
  gl: &'a mut GLContext,
  positions: &TextureUnit,
  depths: &TextureUnit,
  normals: &TextureUnit,
  materials: &TextureUnit,
) -> T<'b> {
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
        } else {
          gl_Position = vec4(0, 0, 0, 1);
        }
      }".to_owned()),
    (gl::FRAGMENT_SHADER, format!("
      #version 330 core

      uniform struct Sun {{
        vec3 direction;
        vec3 intensity;
      }} sun;

      uniform vec3 ambient_light;

      uniform sampler2D positions;
      uniform sampler2D depths;
      uniform sampler2D normals;
      uniform isampler2D materials;

      layout (location = 0) out vec4 frag_color;

      const int WINDOW_WIDTH = {};
      const int WINDOW_HEIGHT = {};

      // include cnoise
      {}

      vec3 grass(vec3 world_position) {{
        {}
      }}

      vec3 dirt(vec3 world_position) {{
        {}
      }}

      vec3 bark(vec3 world_position) {{
        {}
      }}

      vec3 leaves(vec3 world_position) {{
        {}
      }}

      vec3 stone(vec3 world_position) {{
        {}
      }}

      void main() {{
        vec4 base_color;

        vec2 tex_coord = gl_FragCoord.xy / vec2(WINDOW_WIDTH, WINDOW_HEIGHT);
        vec3 world_position = texture(positions, tex_coord).rgb;
        vec3 normal = texture(normals, tex_coord).rgb;
        float depth = texture(depths, tex_coord).r;
        int material = texture(materials, tex_coord).r;

        if (material == 1) {{
          float grass_amount =
            (cnoise(world_position / 32) + 1) / 2 *
            dot(normal, vec3(0, 1, 0)) *
            1.5;
          grass_amount = clamp(grass_amount, 0, 1);
          base_color = vec4(mix(dirt(world_position), grass(world_position), grass_amount), 1);
        }} else if (material == 2) {{
          base_color = vec4(bark(world_position), 1);
        }} else if (material == 3) {{
          base_color = vec4(leaves(world_position), 1);
        }} else if (material == 4) {{
          base_color = vec4(stone(world_position), 1);
        }} else {{
          base_color = vec4(0.5, 0, 0.5, 0.5);
        }}

        float brightness = dot(normal, sun.direction);
        brightness = clamp(brightness, 0, 1);
        vec3 lighting = brightness * sun.intensity + ambient_light;

        float fog_factor = depth / 768;
        float fog_density = 1 - exp(-fog_factor);
        vec3 fog_color = sun.intensity;
        vec4 fog_component = vec4(fog_color, 1) * fog_density;
        vec4 material_component =
          vec4(clamp(lighting, 0, 1), 1) * base_color * vec4(1, 1, 1, 1 - fog_density);

        frag_color = fog_component + material_component;
        frag_color = frag_color - frag_color + vec4(normal, 1);
      }}",
      view::WINDOW_WIDTH,
      view::WINDOW_HEIGHT,
      ::shaders::noise::cnoise(),
      ::shaders::grass::grass(),
      ::shaders::dirt::dirt(),
      ::shaders::bark::bark(),
      ::shaders::leaves::leaves(),
      ::shaders::stone::stone(),
    )),
  );

  let mut shader = Shader::new(gl, components.into_iter());

  let position_uniform = shader.get_uniform_location("positions");
  let depth_uniform = shader.get_uniform_location("depths");
  let material_uniform = shader.get_uniform_location("materials");
  let normal_uniform = shader.get_uniform_location("normals");

  shader.use_shader(gl);
  unsafe {
    gl::Uniform1i(position_uniform, positions.glsl_id as i32);
    gl::Uniform1i(depth_uniform, depths.glsl_id as i32);
    gl::Uniform1i(material_uniform, materials.glsl_id as i32);
    gl::Uniform1i(normal_uniform, normals.glsl_id as i32);
  }

  T {
    shader: shader,
  }
}
