#version 330 core

uniform struct Sun {
  vec3 direction;
  float angular_radius;
} sun;

uniform vec2 window_size;
uniform mat4 projection_matrix;
uniform vec3 eye_position;

uniform samplerBuffer positions;

in vec3 world_position;
in vec3 vs_normal;
flat in int material;

out vec4 frag_color;

include(pixel.glsl)
include(depth_fog.glsl)
include(scatter.glsl)
include(world_fragment.glsl)
include(noise.glsl)
include(grass.glsl)
include(dirt.glsl)
include(bark.glsl)
include(leaves.glsl)
include(stone.glsl)

// http://www.neilmendoza.com/glsl-rotation-about-an-arbitrary-axis/
mat3 rotationMatrix(vec3 axis, float angle)
{
    axis = normalize(axis);
    float s = sin(angle);
    float c = cos(angle);
    float oc = 1.0 - c;

    return mat3(oc * axis.x * axis.x + c,           oc * axis.x * axis.y - axis.z * s,  oc * axis.z * axis.x + axis.y * s,
                oc * axis.x * axis.y + axis.z * s,  oc * axis.y * axis.y + c,           oc * axis.y * axis.z - axis.x * s,
                oc * axis.z * axis.x - axis.y * s,  oc * axis.y * axis.z + axis.x * s,  oc * axis.z * axis.z + c         );
}

vec3 bump_map(float shallowness, float frequency, vec3 v) {
  vec3 seed = frequency * world_position + vec3(0x123411);
  float p0 = cnoise(seed);
  float d = 0.1;
  float px = cnoise(seed + vec3(d, 0, 0));
  float py = cnoise(seed + vec3(0, d, 0));
  float pz = cnoise(seed + vec3(0, 0, d));
  vec3 r = normalize(vec3(px, py, pz) - vec3(p0));

  vec3 axis = cross(vec3(0, 1, 0), r);
  float c = dot(vec3(0, 1, 0), r);
  return rotationMatrix(axis, acos(c) / shallowness) * v;
}

void main() {
  vec4 base_color;

  vec3 normal = vs_normal;
  float shininess = 100000000;

  if (material == 1) {
    // this is duplicated in the grass billboard shader
    float grassiness =
      (cnoise(world_position / 32) + 1) / 2 *
      dot(normal, vec3(0, 1, 0)) *
      1.5;
    grassiness = clamp(grassiness, 0, 1);
    base_color = vec4(mix(dirt(world_position), grass(world_position), grassiness), 1);
  } else if (material == 2) {
    base_color = vec4(bark(world_position), 1);
  } else if (material == 3) {
    base_color = vec4(leaves(world_position), 1);
  } else if (material == 4) {
    base_color = vec4(stone(world_position), 1);
    normal = bump_map(4, 2, normal);
  } else if (material == 5) {
    base_color = vec4(0, 0, 0, 1);
    shininess = 40;
  } else {
    base_color = vec4(0.5, 0, 0.5, 0.5);
    shininess = 1;
  }

  vec3 look = pixel_direction(projection_matrix, eye_position, window_size, gl_FragCoord.xy);
  vec3 ambient_light = rayleigh_color(sun.direction, look);
  vec3 sun_intensity = scatter_color(sun.direction, sun.angular_radius, look);
  frag_color =
    world_fragment(
      sun.direction,
      sun_intensity,
      normalize(world_position - eye_position),
      ambient_light,
      base_color,
      shininess,
      normal,
      vec4(ambient_light, 1),
      gl_FragCoord.z / gl_FragCoord.w
    );
}
