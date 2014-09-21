#version 330 core

const float wireframe_thickness = 0.01;

uniform struct Light {
   vec3 position;
   vec3 intensity;
} light;

uniform vec3 ambient_light;

uniform sampler2D textures[3];

#if $wireframe$
in vec3 geom_world_position;
in vec2 geom_texture_position;
in vec3 geom_normal;
flat in uint geom_type;
in vec3 geom_vertex_position0;
in vec3 geom_vertex_position1;
in vec3 geom_vertex_position2;
#else
in vec3 vert_world_position;
in vec2 vert_texture_position;
in vec3 vert_normal;
flat in uint vert_type;
#endif

out vec4 frag_color;

void main() {
  #if $wireframe$
    vec3 world_position = geom_world_position;
    vec2 texture_position = geom_texture_position;
    vec3 normal = geom_normal;
    uint type = geom_type;
  #else
    vec3 world_position = vert_world_position;
    vec2 texture_position = vert_texture_position;
    vec3 normal = vert_normal;
    uint type = vert_type;
  #endif

  #if $wireframe$
    float edge_distance;
    vec3 edge;

    edge = geom_vertex_position0 - geom_vertex_position1;
    edge_distance =
      length(cross(edge, geom_vertex_position0 - world_position)) / length(edge)
    ;
    edge = geom_vertex_position1 - geom_vertex_position2;
    edge_distance = min(edge_distance,
      length(cross(edge, geom_vertex_position1 - world_position)) / length(edge)
    );
    edge = geom_vertex_position2 - geom_vertex_position0;
    edge_distance = min(edge_distance,
      length(cross(edge, geom_vertex_position2 - world_position)) / length(edge)
    );

    if(edge_distance > wireframe_thickness) {
      discard;
    }
  #endif

  #if $lighting$
    // vector from this position to the light
    vec3 light_path = light.position - world_position;
    // length(normal) = 1, so don't bother dividing.
    float brightness = dot(normal, light_path) / length(light_path);
    brightness = clamp(brightness, 0, 1);
  #endif

  vec4 base_color = vec4(0);
  if(type == uint(0)) {
    base_color = texture(textures[0], texture_position);
  } else if(type == uint(1)) {
    base_color = texture(textures[1], texture_position);
  } else if(type == uint(2)) {
    base_color = texture(textures[2], texture_position);
  }

  #if $lighting$
    vec3 lighting = brightness * light.intensity + ambient_light;
    frag_color = vec4(clamp(lighting, 0, 1), 1) * base_color;
  #else
    frag_color = base_color;
  #endif
}
