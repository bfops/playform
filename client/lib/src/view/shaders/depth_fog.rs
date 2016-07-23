pub fn to_string() -> String {
  r#"
    float fog_density(float distance) {
      return 1 - exp(-distance / 768);
    }

    vec4 apply_fog(vec4 base_color, vec4 fog_color, float distance) {
      return mix(base_color, fog_color, fog_density(distance));
    }
  "#.to_owned()
}
