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
const float atmos_thickness_ratio = 0.025;
const float atmos_thickness = planet_radius * atmos_thickness_ratio;
// assume exponentials basically disappear within a few "decay steps".
const float scale_height = atmos_thickness / 4;

const float sun_distance = 150000000;

// p should be scaled to planet_scale units.
float atmos_density(vec3 p) {
  float dist_from_surface = length(p - planet_center) - planet_radius;
  return exp(-dist_from_surface / scale_height);
}

float optical_ray_depth(vec3 a, float theta) {
  float c_3 = -5.5539e2;
  float c_2 = 4.2581e3;
  float c_1 = -1.0777e4;
  float c_0 = 9.0507e3;
  float theta_0 = 1;
  float theta_1 = theta_0 * theta;
  float theta_2 = theta_1 * theta;
  float theta_3 = theta_2 * theta;
  return atmos_density(a) * (c_0*theta_0 + c_1*theta_1 + c_2*theta_2 + c_3*theta_3);
}

float optical_depth(vec3 a, vec3 b) {
  vec3 d = normalize(b - a);
  float a_theta;
  float b_theta;
  {
    vec3 a_d = a - planet_center;
    a_theta = 3.14 - acos(dot(a_d, d) / length(a_d));
  }
  {
    vec3 b_d = b - planet_center;
    b_theta = 3.14 - acos(dot(b_d, d) / length(b_d));
  }
  return optical_ray_depth(a, a_theta) - optical_ray_depth(b, b_theta);
}

float phase(float cos_angle, float g) {
  float c = cos_angle;
  float g2 = g*g;
  return 3*(1-g2)*(1+c*c) / (2*(2+g2)*pow(1+g2-2*g*c, 3.0/2.0));
}

float in_scatter(vec3 camera, vec3 look, float k, float g) {
  vec3 sun_position = planet_center + sun.direction * sun_distance;

  const int samples = 5;
  const float l = atmos_thickness / samples;
  float r = 0;
  for (int i = 1; i <= samples; ++i) {
    vec3 point = camera + look * i * l;
    float out_scattering = 4*3.14 * k * (optical_depth(camera, point) + optical_depth(point, sun_position));
    float cos_angle = dot(sun_position - camera, camera - point) / (length(camera - point) * length(sun_position - camera));
    r += phase(cos_angle, g) * atmos_density(point) * exp(-out_scattering) * l;
  }
  return k * r;
}

vec3 sky_color(vec3 position, vec3 look) {
  float red = 650;
  float green = 510;
  float blue = 470;

  float red_k = 0.00018;
  float green_k = red_k * pow(green/red, -4);
  float blue_k = red_k * pow(blue/red, -4);

  position /= planet_scale;
  position = position - position + vec3(0, planet_center.y+planet_radius, 0);

  return 20 * (
    vec3(
      in_scatter(position, look, red_k, 0),
      in_scatter(position, look, green_k, 0),
      in_scatter(position, look, blue_k, 0)
    ) +
    vec3(
      in_scatter(position, look, 0.001, -0.999),
      in_scatter(position, look, 0.001, -0.999),
      in_scatter(position, look, 0.001, -0.999)
    )
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
