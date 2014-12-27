#version 330 core

uniform mat4 projection_matrix;

uniform samplerBuffer positions;
uniform samplerBuffer normals;

flat out int vertex_id;
out vec3 normal;

void main() {
  // Mutiply by 3 because there are 3 components for each normal vector.
  int position_id = gl_VertexID * 3;
  vec3 world_position;
  world_position.x = texelFetch(positions, position_id).r;
  world_position.y = texelFetch(positions, position_id + 1).r;
  world_position.z = texelFetch(positions, position_id + 2).r;

  #if $lighting$
    int normal_id = position_id;
    normal.x = texelFetch(normals, normal_id).r;
    normal.y = texelFetch(normals, normal_id + 1).r;
    normal.z = texelFetch(normals, normal_id + 2).r;
  #endif

  vertex_id = gl_VertexID;

  gl_Position = projection_matrix * vec4(world_position, 1.0);
}
