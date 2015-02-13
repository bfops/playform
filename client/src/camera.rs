//! Camera structure and manipulation functions.

use common::matrix;
use gl;
use gl::types::*;
use nalgebra::{Mat4, Vec3, Pnt3};
use nalgebra;
use std::f32::consts::PI;
use std::mem;
use yaglw::gl_context::GLContext;
use yaglw::shader::Shader;

/// Camera representation as 3 distinct matrices, as well as a position + two rotations.
pub struct Camera {
  #[allow(missing_docs)]
  pub position: Pnt3<f32>,
  #[allow(missing_docs)]
  pub lateral_rotation: f32,
  #[allow(missing_docs)]
  pub vertical_rotation: f32,

  // Projection matrix components

  #[allow(missing_docs)]
  pub translation: Mat4<GLfloat>,
  #[allow(missing_docs)]
  pub rotation: Mat4<GLfloat>,
  #[allow(missing_docs)]
  pub fov: Mat4<GLfloat>,
}

impl Camera {
  /// This Camera sits at (0, 0, 0),
  /// maps [-1, 1] in x horizontally,
  /// maps [-1, 1] in y vertically,
  /// and [0, -1] in z in depth.
  pub fn unit() -> Camera {
    Camera {
      position: Pnt3::new(0.0, 0.0, 0.0),
      lateral_rotation: 0.0,
      vertical_rotation: 0.0,

      translation: nalgebra::new_identity(4),
      rotation: nalgebra::new_identity(4),
      fov: nalgebra::new_identity(4),
    }
  }

  #[allow(missing_docs)]
  pub fn projection_matrix(&self) -> Mat4<GLfloat> {
    self.fov * self.rotation * self.translation
  }

  #[allow(missing_docs)]
  pub fn translate_to(&mut self, p: Pnt3<f32>) {
    self.position = p;
    self.translation = matrix::translation(-p.to_vec());
  }

  /// Rotate about a given vector, by `r` radians.
  pub fn rotate(&mut self, v: Vec3<f32>, r: f32) {
    self.rotation = self.rotation * matrix::from_axis_angle4(v, -r);
  }

  /// Rotate the camera around the y axis, by `r` radians. Positive is counterclockwise.
  pub fn rotate_lateral(&mut self, r: GLfloat) {
    self.lateral_rotation = self.lateral_rotation + r;
    self.rotate(Vec3::new(0.0, 1.0, 0.0), r);
  }

  /// Changes the camera pitch by `r` radians. Positive is up.
  /// Angles that "flip around" (i.e. looking too far up or down)
  /// are sliently rejected.
  pub fn rotate_vertical(&mut self, r: GLfloat) {
    let new_rotation = self.vertical_rotation + r;

    if new_rotation < -PI / 2.0
    || new_rotation >  PI / 2.0 {
      return
    }

    self.vertical_rotation = new_rotation;

    let axis =
      matrix::from_axis_angle3(Vec3::new(0.0, 1.0, 0.0), self.lateral_rotation) *
      Vec3::new(1.0, 0.0, 0.0);
    self.rotate(axis, r);
  }
}

/// Set a shader's projection matrix to match that of a camera.
pub fn set_camera(shader: &mut Shader, gl: &mut GLContext, c: &Camera) {
  let projection_matrix = shader.get_uniform_location("projection_matrix");
  shader.use_shader(gl);
  unsafe {
    let val = c.projection_matrix();
    let ptr = mem::transmute(&val);
    gl::UniformMatrix4fv(projection_matrix, 1, 0, ptr);
  }
}
