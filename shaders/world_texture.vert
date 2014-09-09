#version 330 core

uniform mat4 projection_matrix;

uniform vec2 texture_positions[36];
uniform vec3 normals[6];

in vec3 position;
in uint block_type;

out vec2 texture_position;
out vec3 world_position;
out vec3 normal;
flat out uint type;

void main() {
  world_position = position;
  int id = gl_VertexID % 36;
  texture_position = texture_positions[id];
  normal = normals[id / 6];
  type = block_type;

  gl_Position = projection_matrix * vec4(position, 1.0);
}
