vec3 rayleigh_color(vec3 sun_direction, vec3 look) {
  vec3 sun =
    vec3(
      exp(-2 * sun_direction.y),
      1 / (exp(-sun_direction.y) + 1),
      1 / (exp(-sun_direction.y) + 1)
    );
  vec3 sky =
    vec3(
      0.5 * exp(-8 * sun_direction.y * sun_direction.y),
      1 - 0.6 * exp(-2.0 * sun_direction.y),
      1 - 0.4 * exp(-sun_direction.y)
    );

  float cos_angle = dot(sun_direction, look);
  float sin_angle2 = 1 - cos_angle*cos_angle;
  float c2 = 2 - 2*cos_angle;
  float sun_amount = sqrt(sin_angle2 / c2);

  return mix(sky, sun, sun_amount);
}

vec3 scatter_color(vec3 sun_direction, float sun_angular_radius, vec3 look) {
  float mieness = exp(64 * (dot(sun_direction, look) - cos(sun_angular_radius)));
  return mix(rayleigh_color(sun_direction, look), vec3(1), mieness);
}
