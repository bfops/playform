use cgmath::{Point, Point3, Vector3, EuclideanVector};

mod voxel {
  pub use super::super::super::*;
}

#[derive(Debug, Clone, Copy)]
pub struct T {
  pub center: Point3<f32>,
  pub radius: f32,
  pub material: ::voxel::Material,
}

unsafe impl Send for T {}

fn signed_density(this: &T, p: &Point3<f32>) -> f32 {
  let d = this.center.sub_p(p);
  this.radius*this.radius - (d.x*d.x + d.y*d.y + d.z*d.z)
}

impl ::voxel::field::T for T {
  fn density(this: &Self, p: &Point3<f32>) -> f32 {
    signed_density(this, p).abs()
  }

  fn normal(this: &Self, p: &Point3<f32>) -> Vector3<f32> {
    p.sub_p(&this.center).normalize()
  }
}

impl ::voxel::mosaic::T for T {
  fn material(this: &Self, p: &Point3<f32>) -> Option<::voxel::Material> {
    if signed_density(this, p) >= 0.0 {
      Some(this.material)
    } else {
      None
    }
  }
}
