//! Common matrix generation and manipulation functions.

use nalgebra::{Mat3, Mat4, Vec3};
use nalgebra;
use std::num::Float;

/// Create a 3D translation matrix.
pub fn translation(t: Vec3<f32>) -> Mat4<f32> {
  Mat4 {
    m11: 1.0, m12: 0.0, m13: 0.0, m14: t.x,
    m21: 0.0, m22: 1.0, m23: 0.0, m24: t.y,
    m31: 0.0, m32: 0.0, m33: 1.0, m34: t.z,
    m41: 0.0, m42: 0.0, m43: 0.0, m44: 1.0,
  }
}

#[allow(missing_docs)]
pub fn from_axis_angle3(axis: Vec3<f32>, angle: f32) -> Mat3<f32> {
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
pub fn from_axis_angle4(axis: Vec3<f32>, angle: f32) -> Mat4<f32> {
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
pub fn perspective(fovy: f32, aspect: f32, near: f32, far: f32) -> Mat4<f32> {
  Mat4 {
    m11: fovy / aspect, m12: 0.0,   m13: 0.0,                         m14: 0.0,
    m21: 0.0,           m22: fovy,  m23: 0.0,                         m24: 0.0,
    m31: 0.0,           m32: 0.0,   m33: (near + far) / (near - far), m34: 2.0 * near * far / (near - far),
    m41: 0.0,           m42: 0.0,   m43: -1.0,                        m44: 0.0,
  }
}

#[allow(missing_docs)]
pub fn ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Mat4<f32> {
  Mat4 {
    m11: 2.0 / (right - left),  m12: 0.0,                   m13: 0.0,                 m14: (left + right) / (left - right),
    m21: 0.0,                   m22: 2.0 / (top - bottom),  m23: 0.0,                 m24: (bottom + top) / (bottom - top),
    m31: 0.0,                   m32: 0.0,                   m33: 2.0 / (near - far),  m34: (near + far) / (near - far),
    m41: 0.0,                   m42: 0.0,                   m43: 0.0,                 m44: 1.0,
  }
}

/// Create a XY symmetric ortho matrix.
pub fn sortho(dx: f32, dy: f32, near: f32, far: f32) -> Mat4<f32> {
  Mat4 {
    m11: 1.0 / dx,  m12: 0.0,       m13: 0.0,                 m14: 0.0,
    m21: 0.0,       m22: 1.0 / dy,  m23: 0.0,                 m24: 0.0,
    m31: 0.0,       m32: 0.0,       m33: 2.0 / (near - far),  m34: (near + far) / (near - far),
    m41: 0.0,       m42: 0.0,       m43: 0.0,                 m44: 1.0,
  }
}
