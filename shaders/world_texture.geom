#version 330 core

layout(triangles) in;
layout(triangle_strip, max_vertices=3) out;

in vec3 vert_world_position[3];
in vec2 vert_texture_position[3];
in vec3 vert_normal[3];
flat in uint vert_type[3];

out vec3 geom_world_position;
out vec2 geom_texture_position;
out vec3 geom_normal;
flat out uint geom_type;
// arrays out of geometry shaders don't seem to work right.
out vec3 geom_vertex_position0;
out vec3 geom_vertex_position1;
out vec3 geom_vertex_position2;

void main() {
  geom_texture_position = vert_texture_position[0];
  geom_world_position = vert_world_position[0];
  geom_normal = vert_normal[0];
  geom_type = vert_type[0];
  gl_Position = gl_in[0].gl_Position;

  geom_vertex_position0 = vert_world_position[0];
  geom_vertex_position1 = vert_world_position[1];
  geom_vertex_position2 = vert_world_position[2];

  EmitVertex();

  geom_texture_position = vert_texture_position[1];
  geom_world_position = vert_world_position[1];
  geom_normal = vert_normal[1];
  geom_type = vert_type[1];
  gl_Position = gl_in[1].gl_Position;

  geom_vertex_position0 = vert_world_position[0];
  geom_vertex_position1 = vert_world_position[1];
  geom_vertex_position2 = vert_world_position[2];

  EmitVertex();

  geom_texture_position = vert_texture_position[2];
  geom_world_position = vert_world_position[2];
  geom_normal = vert_normal[2];
  geom_type = vert_type[2];
  gl_Position = gl_in[2].gl_Position;

  geom_vertex_position0 = vert_world_position[0];
  geom_vertex_position1 = vert_world_position[1];
  geom_vertex_position2 = vert_world_position[2];

  EmitVertex();

  EndPrimitive();
}
