#version 330 core
in vec2 tex_position;
in vec3 world_position;
in vec3 normal;
out vec4 frag_color;

uniform struct Light {
   vec3 position;
   vec3 intensity;
} light;

uniform vec3 ambient_light;

uniform sampler2D texture_in;

void main(){
  // vector from this position to the light
  vec3 light_path = light.position - world_position;
  // length(normal) = 1, so don't bother dividing.
  float brightness = dot(normal, light_path) / length(light_path);
  brightness = clamp(brightness, 0, 1);
  vec4 base_color = texture(texture_in, vec2(tex_position.x, 1.0 - tex_position.y));
  vec3 lighting = brightness * light.intensity + ambient_light;
  frag_color = vec4(clamp(lighting, 0, 1), 1) * base_color;
}
