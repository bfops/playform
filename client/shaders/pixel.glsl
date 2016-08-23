vec3 pixel_direction(mat4 projection_matrix, vec3 eye_position, vec2 window_size, vec2 pixel) {
  // Scale to [0, 1]
  pixel /= window_size;
  // Scale to [-1, 1]
  pixel = 2*pixel - 1;
  vec4 p = inverse(projection_matrix) * vec4(pixel, -1, 1);
  return normalize(vec3(p / p.w) - eye_position);
}
