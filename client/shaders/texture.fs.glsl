#version 330 core

uniform sampler2D texture_in;
uniform float alpha_threshold;

in vec2 tex_position;

out vec4 frag_color;

void main() {
  vec4 c = texture(texture_in, tex_position);
  if (c.a < alpha_threshold) {
    discard;
  }
  frag_color = c;
}
