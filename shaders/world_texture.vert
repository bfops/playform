#version 330 core

uniform mat4 projection_matrix;

uniform vec2 texture_positions[36];
uniform vec3 normals[6];

in vec3 position;
in uint block_type;

out vec3 vert_world_position;
out vec2 vert_texture_position;
out vec3 vert_normal;
flat out uint vert_type;

void main() {
  int id = gl_VertexID % 36;
  vert_world_position = position;
  vert_texture_position = texture_positions[id];
  vert_type = block_type;

  #if $lighting$
    vert_normal = normals[id / 6];
  #endif

  gl_Position = projection_matrix * vec4(position, 1.0);
}
