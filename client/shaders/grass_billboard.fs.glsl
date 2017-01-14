#version 330 core

include(depth_fog.glsl)
include(world_fragment.glsl)

uniform struct Sun {
  vec3 direction;
  vec3 intensity;
} sun;

uniform vec3 ambient_light;
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
}
