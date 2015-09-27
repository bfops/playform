//! A mosaic with only a single material.

use cgmath::{Point3, Vector3};

use field;
use mosaic;

#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub struct T<Material, Field> {
  pub field: Field,
  pub material: Material,
}

unsafe impl<Material, Field> Send for T<Material, Field> where Field: Send {}

impl<Material, Field> field::T for T<Material, Field> where Field: field::T {
  fn density(&self, p: &Point3<f32>) -> f32 {
    field::T::density(&self.field, p)
  }

  fn normal(&self, p: &Point3<f32>) -> Vector3<f32> {
    field::T::normal(&self.field, p)
  }
}

impl<Material, Field> mosaic::T for T<Material, Field> where 
  Field: field::T,
  Material: Clone,
{
  type Material = Material;

  fn material(&self, p: &Point3<f32>) -> Option<Material> {
    if field::T::density(self, p) >= 0.0 {
      Some(self.material.clone())
    } else {
      None
    }
  }
}
