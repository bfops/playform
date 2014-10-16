#version 330 core

uniform mat4 projection_matrix;

uniform vec3 normals[6];

in vec3 position;
in uint terrain_type;

out vec3 world_position;
out vec3 normal;
flat out uint type;

void main() {
  int id = gl_VertexID % 36;
  type = terrain_type;

  #if $lighting$
    world_position = position;
    normal = normals[id / 6];
  #endif

  gl_Position = projection_matrix * vec4(position, 1.0);
}
