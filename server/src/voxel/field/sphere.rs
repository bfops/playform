use cgmath::{Point, Point3, Vector3, EuclideanVector};

use voxel;

#[derive(Debug, Clone, Copy)]
pub struct T {
  pub center: Point3<f32>,
  pub radius: f32,
}

unsafe impl Send for T {}

impl voxel::field::T for T {
  fn density(this: &Self, p: &Point3<f32>) -> f32 {
    let d = this.center.sub_p(p);
    this.radius*this.radius - (d.x*d.x + d.y*d.y + d.z*d.z)
  }

  fn normal(this: &Self, p: &Point3<f32>) -> Vector3<f32> {
    p.sub_p(&this.center).normalize()
  }
}
