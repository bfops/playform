//! Draw textures using a projection matrix.

use gl;
use yaglw::gl_context::GLContext;
use yaglw::shader::Shader;

use view::shaders;

/// Draw textures using a projection matrix.
pub struct T<'a> {
  #[allow(missing_docs)]
  pub shader: Shader<'a>,
}

#[allow(missing_docs)]
pub fn new<'a, 'b:'a>(gl: &'a GLContext, near: f32, far: f32) -> T<'b> {
  let components = vec!(
    (gl::VERTEX_SHADER, format!(r#"
      #version 330 core

      uniform mat4 projection_matrix;
      uniform vec3 eye_position;
      uniform float time_ms;

      uniform samplerBuffer positions;
      uniform samplerBuffer normals;

      in vec2 texture_position;
      in vec3 vertex_position;
      in vec3 model_translation;
      in int polygon_id;
      in uint tex_id;

      out vec2 vs_texture_position;
      out vec3 vs_normal;
      out float vs_tex_id;

      // include cnoise
      {}

      // include adjust_depth_precision
      {}

      mat3 between(vec3 v1, vec3 v2) {{
        vec3 v = cross(v1, v2);
        float s = length(v);
        float c = dot(v1, v2);
        mat3 skew =
          mat3(
            vec3(0, v.z, -v.y),
            vec3(-v.z, 0, v.x),
            vec3(v.y, -v.x, 0)
          );
        return mat3(1) + skew + skew*skew*(1-c)/(s*s);
      }}

      vec3 vec3Fetch(samplerBuffer vs, int float_idx) {{
        vec3 r;
        r.x = texelFetch(vs, float_idx + 0).r;
        r.y = texelFetch(vs, float_idx + 1).r;
        r.z = texelFetch(vs, float_idx + 2).r;
        return r;
      }}

      // compute a model-space shear to transform (0,1,0) to a desired vector.
      mat4 shearTo(vec3 desired) {{
        mat4 r = mat4(1);

        float d = dot(desired, vec3(0, 1, 0));
        // tweak the dot product to be in a valid range
        float new_d = exp(d - 1.0);
        // the common shear factor
        float k = 0;
        if (d < 0.99) {{
          k = sqrt((1.0-new_d*new_d)/(new_d*new_d*(1.0-d*d)));
        }}

        r[1].x = desired.x * k;
        r[1].y = 1.0;
        r[1].z = desired.z * k;

        return r;
      }}

      void main() {{
        vs_texture_position = texture_position;
        vs_tex_id = float(tex_id);

        // Put the grass tuft in the middle of the underlying terrain polygon.
        int position_id = polygon_id * 3 * 3;
        mat3 vertices =
          mat3(
            vec3Fetch(positions, position_id),
            vec3Fetch(positions, position_id + 3),
            vec3Fetch(positions, position_id + 6)
          );
        vec3 side_length =
          vec3(
            length(vertices[0] - vertices[1]),
            length(vertices[1] - vertices[2]),
            length(vertices[2] - vertices[0])
          );
        vec3 root = vertices * vec3(1.0/3.0);

        // Find the normal for the grass by barycentrically interpolating the
        // vertex normals to the root.
        int normal_id = polygon_id * 3 * 3;
        mat3 vertex_normals =
          mat3(
            vec3Fetch(normals, normal_id),
            vec3Fetch(normals, normal_id + 3),
            vec3Fetch(normals, normal_id + 6)
          );
        vec3 normal = vertex_normals * vec3(1.0/3.0);

        mat4 translation = mat4(1.0);
        translation[3].xyz = root;

        mat3 rotate_normal = between(vec3(0, 1, 0), normal);
        mat4 rotation = mat4(rotate_normal);

        mat4 shear = shearTo(inverse(rotate_normal) * vec3(0, 1, 0));

        mat4 noise_shear;
        {{
          vec3 billboard_seed = root / vec3(1 << 3, 1, 1 << 3);
          float azimuth = 3.14 * cnoise(billboard_seed + vec3(122, -1, 14.5) + vec3(0, time_ms / 2000, 0));
          float altitude = 3.14/2 - 3.14/4 * (cnoise(billboard_seed + vec3(-18.11, 101.1, 44.5) + vec3(0, time_ms / 2000, 0)) + 1) / 2.0;
          vec3 v =
            vec3(
              cos(altitude) * cos(azimuth),
              sin(altitude),
              cos(altitude) * sin(azimuth)
            );
          noise_shear = shearTo(v);
        }}

        // this is duplicated in the terrain shader
        float grassiness =
          (cnoise(root / 32) + 1) / 2 *
          dot(normal, vec3(0, 1, 0)) *
          1.5;
        grassiness = clamp(grassiness, 0, 1);

        mat4 scale = mat4(1.0);
        float max_side = max(max(side_length[0], side_length[1]), side_length[2]);
        float min_side = min(min(side_length[0], side_length[1]), side_length[2]);
        float side_scale = (min_side + max_side) / 2.0;
        scale[1].y = grassiness * 0.8;
        scale[0].x = grassiness * side_scale * 1.0;
        scale[2].z = scale[0].x;

        mat4 model_translation_mat = mat4(1.0);
        model_translation_mat[3].xyz = model_translation;

        mat4 shear_mat = shear * noise_shear;
        mat4 to_world_mat = translation * rotation * model_translation_mat;

        vec4 scaled = scale * vec4(vertex_position, 1);
        vec4 sheared = shear_mat * scaled;
        float length_ratio = length(vec3(scaled)) / length(vec3(sheared));
        sheared *= vec4(vec3(length_ratio), 1);

        gl_Position =
          adjust_depth_precision(
            projection_matrix *
            to_world_mat *
            sheared
          );

        vs_normal = normal;
      }}"#,
      shaders::adjust_depth_precision::as_string(near, far),
      shaders::noise::cnoise(),
    )),
    (gl::FRAGMENT_SHADER, format!("
      #version 330 core

      uniform struct Sun {{
        vec3 direction;
        vec3 intensity;
      }} sun;

      uniform vec3 ambient_light;
      uniform vec3 eye_position;

      uniform sampler2D texture_in;
      uniform float alpha_threshold;

      in vec2 vs_texture_position;
      in vec3 vs_normal;
      in float vs_tex_id;

      out vec4 frag_color;

      // depth fog
      {}

      // world fragment shading
      {}

      void main() {{
        int tex_id = int(round(vs_tex_id));
        int y = tex_id / 3;
        int x = tex_id % 3;
        vec2 tex_position =
          (vs_texture_position + y*vec2(0, 1) + x*vec2(1, 0)) / 3
          - vec2(0.0, 0.05);
        vec4 c = texture(texture_in, tex_position);
        if (c.a < alpha_threshold) {{
          discard;
        }}
        vec3 world_position = vec3(gl_FragCoord.xy * gl_FragCoord.w, gl_FragCoord.w);
        vec4 fog_color = vec4(sun.intensity, 1);
        frag_color =
          world_fragment(
            sun.direction,
            sun.intensity,
            normalize(world_position - eye_position),
            ambient_light,
            c,
            1.0 / 0.0,
            vs_normal,
            fog_color,
            gl_FragCoord.z / gl_FragCoord.w
          );
      }}",
      shaders::depth_fog::to_string(),
      shaders::world_fragment::to_string(),
    )),
  );
  T {
    shader: Shader::new(gl, components.into_iter()),
  }
}
