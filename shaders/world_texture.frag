#version 330 core

uniform struct Light {
   vec3 position;
   vec3 intensity;
} light;

uniform vec3 ambient_light;

uniform sampler2D textures[3];

in vec3 world_position;
in vec2 texture_position;
in vec3 normal;
flat in uint type;

out vec4 frag_color;

void main() {
  $if lighting
    // vector from this position to the light
    vec3 light_path = light.position - world_position;
    // length(normal) = 1, so don't bother dividing.
    float brightness = dot(normal, light_path) / length(light_path);
    brightness = clamp(brightness, 0, 1);
  $end

  vec4 base_color = vec4(0);
  if(type == uint(0)) {
    base_color = texture(textures[0], texture_position);
  } else if(type == uint(1)) {
    base_color = texture(textures[1], texture_position);
  } else if(type == uint(2)) {
    base_color = texture(textures[2], texture_position);
  }

  $if lighting
    vec3 lighting = brightness * light.intensity + ambient_light;
    frag_color = vec4(clamp(lighting, 0, 1), 1) * base_color;
  $else
    frag_color = base_color;
  $end
}
