#version 330 core

include(depth_fog.glsl)
include(world_fragment.glsl)
include(scatter.glsl)
include(pixel.glsl)

uniform struct Sun {
  vec3 direction;
  float angular_radius;
} sun;

uniform vec2 window_size;
uniform mat4 projection_matrix;
uniform vec3 eye_position;

uniform sampler2D texture_in;
uniform float alpha_threshold;

in vec2 vs_texture_position;
in vec3 vs_normal;
in float vs_tex_id;

out vec4 frag_color;

void main() {
  int tex_id = int(round(vs_tex_id));
  int y = tex_id / 3;
  int x = tex_id % 3;
  vec2 tex_position =
    (vs_texture_position + y*vec2(0, 1) + x*vec2(1, 0)) / 3
    - vec2(0.0, 0.05);
  vec4 c = texture(texture_in, tex_position);
  if (c.a < alpha_threshold) {
    discard;
  }
  vec3 world_position = vec3(gl_FragCoord.xy * gl_FragCoord.w, gl_FragCoord.w);
  vec3 look = pixel_direction(projection_matrix, eye_position, window_size, gl_FragCoord.xy);
  vec3 ambient_light = rayleigh_color(sun.direction, look);
  vec3 sun_intensity = scatter_color(sun.direction, sun.angular_radius, look);
  frag_color =
    world_fragment(
      sun.direction,
      sun_intensity,
      normalize(world_position - eye_position),
      ambient_light,
      c,
      1.0 / 0.0,
      vs_normal,
      vec4(ambient_light, 1),
      gl_FragCoord.z / gl_FragCoord.w
    );
}
