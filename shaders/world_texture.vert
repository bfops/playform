#version 330 core

uniform mat4 projection_matrix;

uniform samplerBuffer positions;

flat out int vertex_id;

void main() {
  int position_id = gl_VertexID * 3;
  vec3 world_position;
  world_position.x = texelFetch(positions, position_id).r;
  world_position.y = texelFetch(positions, position_id + 1).r;
  world_position.z = texelFetch(positions, position_id + 2).r;
  vertex_id = gl_VertexID;

  gl_Position = projection_matrix * vec4(world_position, 1.0);
}
