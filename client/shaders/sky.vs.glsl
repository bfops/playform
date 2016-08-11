#version 330 core

void main() {
  if (gl_VertexID == 0) {
    gl_Position = vec4(1, -1, 0, 1);
  } else if (gl_VertexID == 1) {
    gl_Position = vec4(1, 1, 0, 1);
  } else if (gl_VertexID == 2) {
    gl_Position = vec4(-1, -1, 0, 1);
  } else if (gl_VertexID == 3) {
    gl_Position = vec4(-1, 1, 0, 1);
  }
}
