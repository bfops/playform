#version 330 core

include(adjust_depth_precision.glsl)

uniform float near_clip;
uniform float far_clip;
uniform mat4 projection_matrix;

uniform samplerBuffer positions;
uniform samplerBuffer normals;
uniform isamplerBuffer materials;

out vec3 world_position;
out vec3 vs_normal;
flat out int material;

void main() {
  // Mutiply by 3 because there are 3 components for each normal vector.
  int position_id = gl_VertexID * 3;
  world_position.x = texelFetch(positions, position_id + 0).r;
  world_position.y = texelFetch(positions, position_id + 1).r;
  world_position.z = texelFetch(positions, position_id + 2).r;

  int normal_id = position_id;
  vs_normal.x = texelFetch(normals, normal_id + 0).r;
  vs_normal.y = texelFetch(normals, normal_id + 1).r;
  vs_normal.z = texelFetch(normals, normal_id + 2).r;

  int face_id = gl_VertexID / 3;

  material = texelFetch(materials, face_id).r;

  gl_Position = adjust_depth_precision(near_clip, far_clip, projection_matrix * vec4(world_position, 1.0));
}
