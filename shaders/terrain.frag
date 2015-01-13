#version 330 core

uniform struct Light {
   vec3 position;
   vec3 intensity;
} light;

uniform vec3 ambient_light;

uniform samplerBuffer positions;
uniform isamplerBuffer terrain_types;

flat in int vertex_id;
in vec3 normal;

out vec4 frag_color;

void main() {
  int face_id = vertex_id / 3;

  uint terrain_type = uint(texelFetch(terrain_types, face_id).r);

  #if $lighting$
    // Mutiply by 3 because there are 3 components for each position vector.
    int position_id = vertex_id * 3;
    vec3 world_position;
    world_position.x = texelFetch(positions, position_id).r;
    world_position.y = texelFetch(positions, position_id + 1).r;
    world_position.z = texelFetch(positions, position_id + 2).r;

    // vector from here to the light
    vec3 light_path = light.position - world_position;
    light_path = normalize(light_path);
    // length(normal) = 1 already.
    float brightness = dot(normal, light_path);
    brightness = clamp(brightness, 0, 1);
  #endif

  vec4 base_color;
  if(terrain_type == uint(0)) {
    base_color = vec4(0, 0.5, 0, 1);
  } else if(terrain_type == uint(1)) {
    base_color = vec4(0.5, 0.4, 0.2, 1);
  } else if(terrain_type == uint(2)) {
    base_color = vec4(0.5, 0.5, 0.5, 1);
  } else {
    base_color = vec4(float(terrain_type) / 65535, 0, 0, 1);
  }

  #if $lighting$
    vec3 lighting = brightness * light.intensity + ambient_light;
    frag_color = vec4(clamp(lighting, 0, 1), 1) * base_color;
  #else
    frag_color = base_color;
  #endif
}
