use cgmath::{Point, Point3, Vector3, EuclideanVector};

use voxel;

#[derive(Debug, Clone, Copy)]
pub struct T {
  pub radius: f32,
}

unsafe impl Send for T {}

impl voxel::field::T for T {
  fn density(this: &Self, p: &Point3<f32>) -> f32 {
    this.radius*this.radius - p.to_vec().length2()
  }

  fn normal(_: &Self, p: &Point3<f32>) -> Vector3<f32> {
    p.to_vec().normalize()
  }
}
