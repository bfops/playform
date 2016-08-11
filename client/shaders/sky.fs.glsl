#version 330 core

include(depth_fog.glsl)
include(noise.glsl)

uniform vec2 window_size;

uniform struct Sun {
  vec3 direction;
  vec3 intensity;
} sun;

const float sun_angular_radius = 3.14/32;

uniform mat4 projection_matrix;
uniform vec3 eye_position;

uniform float time_ms;

out vec4 frag_color;

vec3 pixel_direction(vec2 pixel) {
  // Scale to [0, 1]
  pixel /= window_size;
  // Scale to [-1, 1]
  pixel = 2*pixel - 1;
  vec4 p = inverse(projection_matrix) * vec4(pixel, -1, 1);
  return normalize(vec3(p / p.w) - eye_position);
}

float cloud_noise(vec3 seed) {
  float f = cnoise(seed + vec3(0, time_ms / 8000, 0));
  return f;
}

float cloud_density(vec3 seed) {
  float d = (2.0*cloud_noise(seed / 2) + cloud_noise(seed) + 0.5*cloud_noise(2.0 * seed) + 0.25*cloud_noise(4.0*seed)) / 3.75;
  d = (d + 1) / 2;
  return d;
}

void main() {
  vec3 direction = pixel_direction(gl_FragCoord.xy);

  const int HEIGHTS = 2;
  float heights[HEIGHTS] = float[](150, 1000);
  vec3 offsets[HEIGHTS] = vec3[](vec3(12,553,239), vec3(-10, 103, 10004));

  vec3 c = vec3(0);
  float alpha = 1;
  for (int i = 0; i < HEIGHTS; ++i) {
    float cloud_height = heights[i];
    float dist = (cloud_height - eye_position.y) / direction.y;
    if (dist <= 0 || dist > 1000000) {
      continue;
    } else {
      vec3 seed = (eye_position + dist * direction + offsets[i]) / 1000 * vec3(1, 4, 1);

      float depth_alpha = fog_density(dist / 16);

      float density = cloud_density(seed);

      float cloud_alpha = density;
      float min_cloud = 0.4;
      float max_cloud = 0.8;
      cloud_alpha = (cloud_alpha - min_cloud) / (max_cloud - min_cloud);
      cloud_alpha = min(max(cloud_alpha, 0), 1);
      cloud_alpha *= (1 - depth_alpha);

      float lightness = pow(max(density - cloud_density(seed + 10 * sun.direction), 0), 1.0) * (1 - density);
      vec3 cloud_color = vec3(mix(0.4, 1, lightness));
      c += alpha * cloud_alpha * cloud_color;
      alpha *= (1 - cloud_alpha);
    }
  }

  float sunniness = exp(64 * (dot(sun.direction, direction) - cos(sun_angular_radius)));
  vec3 infinity_color = mix(sun.intensity, vec3(1), sunniness);
  c += alpha * infinity_color;

  frag_color = min(vec4(c, 1), vec4(1));
}
