#version 330 core

include(adjust_depth_precision.glsl)

uniform mat4 projection_matrix;
uniform float near_clip;
uniform float far_clip;

in vec3 position;
in vec4 in_color;

out vec4 color;

void main() {
  gl_Position = adjust_depth_precision(near_clip, far_clip, projection_matrix * vec4(position, 1.0));
  color = in_color;
}
