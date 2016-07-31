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

      uniform samplerBuffer positions;
      uniform samplerBuffer normals;

      in vec2 texture_position;
      in vec3 vertex_position;
      in int polygon_id;
      in uint tex_id;

      out vec2 vs_texture_position;
      out vec3 vs_normal;
      out float vs_tex_id;

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

      // Barycentric interpolation function to smoothly interpolate vertex-associated values over a triangle.
      // This returns the weight that should be given to each vertex value at a specified point.
      vec3 barycentric(mat3 v, vec3 p) {{
        mat3 d = v - mat3(p, p, p);
        vec3 weights =
          vec3(
            length(cross(d[1], d[2])),
            length(cross(d[2], d[0])),
            length(cross(d[0], d[1]))
          );
        return weights / dot(weights, vec3(1));
      }}

      vec3 vec3Fetch(samplerBuffer vs, int float_idx) {{
        vec3 r;
        r.x = texelFetch(vs, float_idx + 0).r;
        r.y = texelFetch(vs, float_idx + 1).r;
        r.z = texelFetch(vs, float_idx + 2).r;
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
        vec3 normal = vertex_normals * barycentric(vertices, root);

        mat4 translation = mat4(1.0);
        translation[3].xyz = root;

        mat3 rotate_normal = between(vec3(0, 1, 0), normal);
        mat4 rotation = mat4(rotate_normal);

        // We will apply a model-space skew transformation to make the grass point
        // in a desired direction. In model coordinates the grass initially points
        // to (0, 1, 0), so to skew this into (xd, yd, zd) we apply a skew of
        //    x' = x0 + xd*(y/yd)
        //    y' = y
        //    z' = z0 + zd*(y/yd)
        mat4 skew = mat4(1.0);
        {{
          vec3 desired = vec3(0, 1, 0);
          float d = dot(normal, desired);
          if (d < 0.99) {{
            // in model space
            vec3 desired = inverse(rotate_normal) * desired;
            // tweak the dot product to be in a valid range
            float new_d = exp(d - 1.0);
            // the common skew factor
            float k = sqrt((1.0-new_d*new_d)/(new_d*new_d*(1.0-d*d)));

            skew[1].x = desired.x * k;
            skew[1].y = 1.0;
            skew[1].z = desired.z * k;
          }}
        }};

        mat4 scale = mat4(1.0);
        scale[0].x = scale[2].z = 4.0;

        mat4 model_matrix = translation * rotation * skew * scale;

        gl_Position =
          adjust_depth_precision(
            projection_matrix *
            model_matrix *
            vec4(vertex_position, 1)
          );

        vs_normal = normal;
      }}"#,
      shaders::adjust_depth_precision::as_string(near, far),
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
        vec2 tex_position = (vs_texture_position + y*vec2(0, 1) + x*vec2(1, 0)) / 3;
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
