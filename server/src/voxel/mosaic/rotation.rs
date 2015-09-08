use cgmath::{Point3, Vector3, Rotation, Basis3};

use voxel::{field, mosaic};

#[derive(Clone)]
pub struct T<Mosaic> {
  pub rotation: Basis3<f32>,
  pub mosaic: Mosaic,
}

impl<Mosaic> field::T for T<Mosaic> where Mosaic: field::T {
  fn density(&self, p: &Point3<f32>) -> f32 {
    let p = self.rotation.invert().rotate_point(p);
    field::T::density(&self.mosaic, &p)
  }

  fn normal(&self, p: &Point3<f32>) -> Vector3<f32> {
    let p = self.rotation.invert().rotate_point(p);
    field::T::normal(&self.mosaic, &p)
  }
}

impl<Mosaic> mosaic::T for T<Mosaic> where Mosaic: mosaic::T {
  fn density(&self, p: &Point3<f32>) -> f32 {
    let p = self.rotation.invert().rotate_point(p);
    mosaic::T::density(&self.mosaic, &p)
  }

  fn material(&self, p: &Point3<f32>) -> Option<::voxel::Material> {
    let p = self.rotation.invert().rotate_point(p);
    mosaic::T::material(&self.mosaic, &p)
  }
}
