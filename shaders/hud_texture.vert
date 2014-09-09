#version 330 core

uniform mat4 projection_matrix;

in vec3 position;
in vec2 texture_position;

out vec2 tex_position;

void main() {
  tex_position = texture_position;
  gl_Position = projection_matrix * vec4(position, 1.0);
}
