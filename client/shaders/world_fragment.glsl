vec4 world_fragment(
  vec3 light_direction,
  vec3 intensity,
  vec3 view_direction,
  vec3 ambient_light,
  vec4 material_color,
  float shininess,
  vec3 normal,
  vec4 fog_color,
  float frag_distance
) {
  float diffuse = dot(normal, light_direction);
  diffuse = clamp(diffuse, 0, 1);

  vec3 reflected = view_direction - 2*dot(view_direction, normal)*normal;
  float specular = dot(reflected, light_direction);
  specular = clamp(specular, 0, 1);
  specular = pow(specular, shininess);

  vec4 with_light = diffuse*vec4(intensity, 1)*material_color + specular*vec4(1) + vec4(ambient_light, 1)*material_color;
  return apply_fog(with_light, fog_color, frag_distance);
}
