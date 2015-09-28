//! A sphere field

use cgmath::{Point, Point3, Vector3, EuclideanVector};

use field;

#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct T {
  pub radius: f32,
}

unsafe impl Send for T {}

impl field::T for T {
  fn density(&self, p: &Point3<f32>) -> f32 {
    self.radius*self.radius - p.to_vec().length2()
  }

  fn normal(&self, p: &Point3<f32>) -> Vector3<f32> {
    p.to_vec().normalize()
  }
}
