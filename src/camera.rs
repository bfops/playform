use gl;
use gl::types::*;
use nalgebra::{Mat3, Mat4, Vec3, Pnt3};
use nalgebra;
use std::f32::consts::PI;
use std::mem;
use std::num::Float;
use yaglw::gl_context::GLContext;
use yaglw::shader::Shader;

pub struct Camera {
  pub position: Pnt3<f32>,
  pub lateral_rotation: f32,
  pub vertical_rotation: f32,

  // projection matrix components
  pub translation: Mat4<GLfloat>,
  pub rotation: Mat4<GLfloat>,
  pub fov: Mat4<GLfloat>,
}

/// Create a 3D translation matrix.
pub fn translation(t: Vec3<GLfloat>) -> Mat4<GLfloat> {
  Mat4 {
    m11: 1.0, m12: 0.0, m13: 0.0, m14: t.x,
    m21: 0.0, m22: 1.0, m23: 0.0, m24: t.y,
    m31: 0.0, m32: 0.0, m33: 1.0, m34: t.z,
    m41: 0.0, m42: 0.0, m43: 0.0, m44: 1.0,
  }
}

pub fn from_axis_angle3(axis: Vec3<GLfloat>, angle: GLfloat) -> Mat3<GLfloat> {
  let (s, c) = angle.sin_cos();
  let Vec3 { x: xs, y: ys, z: zs } = axis * s;
  let vsub1c = axis * (1.0 - c);
  nalgebra::outer(&vsub1c, &vsub1c) +
    Mat3 {
      m11: c,   m12: -zs, m13: ys,
      m21: zs,  m22: c,   m23: -xs,
      m31: -ys, m32: xs,  m33: c,
    }
}

/// Create a matrix from a rotation around an arbitrary axis.
pub fn from_axis_angle4(axis: Vec3<GLfloat>, angle: GLfloat) -> Mat4<GLfloat> {
  let (s, c) = angle.sin_cos();
  let sub1c = 1.0 - c;
  let Vec3 { x: xs, y: ys, z: zs } = axis * s;
  let (x, y, z) = (axis.x, axis.y, axis.z);
  Mat4 {
    m11: x*x*sub1c + c,  m12: x*y*sub1c - zs, m13: x*z*sub1c + ys, m14: 0.0,
    m21: y*x*sub1c + zs, m22: y*y*sub1c + c,  m23: y*z*sub1c - xs, m24: 0.0,
    m31: z*x*sub1c - ys, m32: z*y*sub1c + xs, m33: z*z*sub1c + c,  m34: 0.0,
    m41: 0.0,            m42: 0.0,            m43: 0.0,            m44: 1.0,
  }
}

/// Create a 3D perspective initialization matrix.
pub fn perspective(fovy: GLfloat, aspect: GLfloat, near: GLfloat, far: GLfloat) -> Mat4<GLfloat> {
  Mat4 {
    m11: fovy / aspect, m12: 0.0,   m13: 0.0,                         m14: 0.0,
    m21: 0.0,           m22: fovy,  m23: 0.0,                         m24: 0.0,
    m31: 0.0,           m32: 0.0,   m33: (near + far) / (near - far), m34: 2.0 * near * far / (near - far),
    m41: 0.0,           m42: 0.0,   m43: -1.0,                        m44: 0.0,
  }
}

#[allow(dead_code)]
pub fn ortho(left: GLfloat, right: GLfloat, bottom: GLfloat, top: GLfloat, near: GLfloat, far: GLfloat) -> Mat4<GLfloat> {
  Mat4 {
    m11: 2.0 / (right - left),  m12: 0.0,                   m13: 0.0,                 m14: (left + right) / (left - right),
    m21: 0.0,                   m22: 2.0 / (top - bottom),  m23: 0.0,                 m24: (bottom + top) / (bottom - top),
    m31: 0.0,                   m32: 0.0,                   m33: 2.0 / (near - far),  m34: (near + far) / (near - far),
    m41: 0.0,                   m42: 0.0,                   m43: 0.0,                 m44: 1.0,
  }
}

/// Create a XY symmetric ortho matrix.
pub fn sortho(dx: GLfloat, dy: GLfloat, near: GLfloat, far: GLfloat) -> Mat4<GLfloat> {
  Mat4 {
    m11: 1.0 / dx,  m12: 0.0,       m13: 0.0,                 m14: 0.0,
    m21: 0.0,       m22: 1.0 / dy,  m23: 0.0,                 m24: 0.0,
    m31: 0.0,       m32: 0.0,       m33: 2.0 / (near - far),  m34: (near + far) / (near - far),
    m41: 0.0,       m42: 0.0,       m43: 0.0,                 m44: 1.0,
  }
}

impl Camera {
  /// this Camera sits at (0, 0, 0),
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

  pub fn projection_matrix(&self) -> Mat4<GLfloat> {
    self.fov * self.rotation * self.translation
  }

  pub fn translate_to(&mut self, p: Pnt3<f32>) {
    self.position = p;
    self.translation = translation(-p.to_vec());
  }

  /// Rotate about a given vector, by `r` radians.
  pub fn rotate(&mut self, v: Vec3<f32>, r: f32) {
    self.rotation = self.rotation * from_axis_angle4(v, -r);
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
      from_axis_angle3(Vec3::new(0.0, 1.0, 0.0), self.lateral_rotation) *
      Vec3::new(1.0, 0.0, 0.0);
    self.rotate(axis, r);
  }
}

pub fn set_camera(shader: &mut Shader, gl: &mut GLContext, c: &Camera) {
  let projection_matrix = shader.get_uniform_location("projection_matrix");
  shader.use_shader(gl);
  unsafe {
    let val = c.projection_matrix();
    let ptr = mem::transmute(&val);
    gl::UniformMatrix4fv(projection_matrix, 1, 0, ptr);
  }
}
