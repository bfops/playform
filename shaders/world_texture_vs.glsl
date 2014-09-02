#version 330 core
uniform mat4 projection_matrix;
in vec3 position;
in vec2 texture_position;
in vec3 vertex_normal;

out vec2 tex_position;
out vec3 world_position;
out vec3 normal;

void main() {
  tex_position = texture_position;
  world_position = position;
  normal = vertex_normal;

  gl_Position = projection_matrix * vec4(position, 1.0);
}
