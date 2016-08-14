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

// The scattering approach is heavily based on the GPU Gems article:
// http://http.developer.nvidia.com/GPUGems2/gpugems2_chapter16.html.
// a and b should be relative to the center of the atmosphere.

const float planet_scale = 1000;
const float planet_radius = 6400;
// assume (0, 0, 0) is on the surface of the earth.
const vec3 planet_center = vec3(0, -planet_radius, 0);
// thickness of the atmosphere, as a percentage of the planet radius.
const float atmos_thickness_ratio = 0.050;
const float atmos_thickness = planet_radius * atmos_thickness_ratio;
// assume exponentials basically disappear within a few "decay steps".
const float scale_height = atmos_thickness / 8;

// p should be scaled to planet_scale units.
float atmos_density(vec3 p) {
  float dist_from_surface = length(p - planet_center) - planet_radius;
  return exp(-dist_from_surface / scale_height);
}

float optical_depth(vec3 a, vec3 b) {
  const int samples = 20;
  vec3 d = b - a;
  float l = min(length(d), 2 * (atmos_thickness + planet_radius)) / samples;
  d = normalize(d) * l;
  float r = 0;
  for (int i = 1; i <= samples; ++i) {
    vec3 p = vec3(a + i * d);
    r += atmos_density(p) * l;
  }
  return r;
}

float phase(float cos_angle, float g) {
  float c = cos_angle;
  float g2 = g*g;
  return 3*(1-g2)*(1+c*c) / (2*(2+g2)*pow(1+g2-2*g*c, 3.0/2.0));
}

float in_scatter(vec3 camera, vec3 look, float k, float g) {
  const float sun_distance = 150000000;
  vec3 sun_position = planet_center + sun.direction * sun_distance;

  const int samples = 20;
  const float l = atmos_thickness / samples;
  float r = 0;
  for (int i = 1; i <= samples; ++i) {
    vec3 point = camera + look * i * l;
    float cos_angle = dot(sun_position - point, point - camera) / (length(sun_position - point) * length(point - camera));
    float od = optical_depth(camera, point) + optical_depth(point, sun_position);
    r += k * phase(cos_angle, g) * atmos_density(point) * exp(-k * od) * l;
  }
  return r;
}

vec3 sky_color(vec3 position, vec3 look) {
  position /= planet_scale;
  return
    vec3(
      in_scatter(position, look, 0.04, 0),
      in_scatter(position, look, 0.05, 0),
      in_scatter(position, look, 0.001, 0)
//    ) +
//    vec3(
//      in_scatter(position, look, atmos_center, sun_position, dscale, ascale, 0.4, -0.999),
//      in_scatter(position, look, atmos_center, sun_position, dscale, ascale, 0.4, -0.999),
//      in_scatter(position, look, atmos_center, sun_position, dscale, ascale, 0.4, -0.999)
    );
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

  vec3 infinity_color = sky_color(eye_position, direction);
  c += alpha * infinity_color;
  // This disables clouds.
  c = c - c + infinity_color;

  frag_color = min(vec4(c, 1), vec4(1));
}
