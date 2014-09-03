#version 330 core

uniform sampler2D texture_in;

in vec2 tex_position;

out vec4 frag_color;

void main() {
  frag_color = texture(texture_in, vec2(tex_position.x, 1.0 - tex_position.y));
}
