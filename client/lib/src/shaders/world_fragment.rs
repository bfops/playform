pub fn to_string() -> String {
  r#"
    vec4 world_fragment(
      vec3 light_direction,
      vec3 intensity,
      vec3 ambient_light,
      vec4 material_color,
      vec3 normal,
      vec4 fog_color,
      float frag_distance
    ) {{
      float brightness = dot(normal, light_direction);
      brightness = clamp(brightness, 0, 1);
      vec3 lighting = brightness * intensity + ambient_light;

      material_color *= vec4(clamp(lighting, 0, 1), 1);
      return apply_fog(material_color, fog_color, frag_distance);
    }}
  "#.to_owned()
}
