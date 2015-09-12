use cgmath::{Point3, Vector3, Rotation, Basis3};

use voxel::field;

#[derive(Clone)]
pub struct T<Mosaic> {
  pub rotation: Basis3<f32>,
  pub field: Mosaic,
}

impl<Mosaic> field::T for T<Mosaic> where Mosaic: field::T {
  fn density(&self, p: &Point3<f32>) -> f32 {
    let p = self.rotation.invert().rotate_point(p);
    field::T::density(&self.field, &p)
  }

  fn normal(&self, p: &Point3<f32>) -> Vector3<f32> {
    let p = self.rotation.invert().rotate_point(p);
    field::T::normal(&self.field, &p)
  }
}
