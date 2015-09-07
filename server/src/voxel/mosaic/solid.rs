use cgmath::{Point3, Vector3};

use voxel;

#[derive(Debug, Clone, Copy)]
pub struct T<Field> {
  pub field: Field,
  pub material: voxel::Material,
}

unsafe impl<Field> Send for T<Field> where Field: Send {}

impl<Field> voxel::field::T for T<Field> where Field: voxel::field::T {
  fn density(&self, p: &Point3<f32>) -> f32 {
    voxel::field::T::density(&self.field, p)
  }

  fn normal(&self, p: &Point3<f32>) -> Vector3<f32> {
    voxel::field::T::normal(&self.field, p)
  }
}

impl<Field> voxel::mosaic::T for T<Field> where Field: voxel::field::T {
  fn material(&self, p: &Point3<f32>) -> Option<voxel::Material> {
    if voxel::field::T::density(self, p) >= 0.0 {
      Some(self.material)
    } else {
      None
    }
  }
}
