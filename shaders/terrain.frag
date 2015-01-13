#version 330 core

uniform struct Light {
   vec3 position;
   vec3 intensity;
} light;

uniform vec3 ambient_light;

uniform samplerBuffer positions;
uniform samplerBuffer colors;

flat in int vertex_id;
in vec3 normal;

out vec4 frag_color;

void main() {
  int face_id = vertex_id / 3;
  int color_id = face_id * 3;

  vec4 base_color;
  base_color.x = texelFetch(colors, color_id).r;
  base_color.y = texelFetch(colors, color_id + 1).r;
  base_color.z = texelFetch(colors, color_id + 2).r;
  base_color.w = 1.0;

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

    vec3 lighting = brightness * light.intensity + ambient_light;
    frag_color = vec4(clamp(lighting, 0, 1), 1) * base_color;
  #else
    frag_color = base_color;
  #endif
}
