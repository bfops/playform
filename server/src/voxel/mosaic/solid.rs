use cgmath::{Point3, Vector3};

use voxel;

#[derive(Debug, Clone, Copy)]
pub struct T<Field> {
  pub field: Field,
  pub material: voxel::Material,
}

unsafe impl<Field> Send for T<Field> where Field: Send {}

impl<Field> voxel::field::T for T<Field> where Field: voxel::field::T {
  fn density(this: &Self, p: &Point3<f32>) -> f32 {
    voxel::field::T::density(&this.field, p)
  }

  fn normal(this: &Self, p: &Point3<f32>) -> Vector3<f32> {
    voxel::field::T::normal(&this.field, p)
  }
}

impl<Field> voxel::mosaic::T for T<Field> where Field: voxel::field::T {
  fn material(this: &Self, p: &Point3<f32>) -> Option<voxel::Material> {
    if voxel::field::T::density(this, p) >= 0.0 {
      Some(this.material)
    } else {
      None
    }
  }
}
