use cgmath::{Point, Point3, Vector3};

use voxel::field;

#[derive(Debug, Clone, Copy)]
pub struct T<Field> {
  pub translation: Vector3<f32>,
  pub field: Field,
}

impl<Field> field::T for T<Field> where Field: field::T {
  fn density(this: &Self, p: &Point3<f32>) -> f32 {
    let p = p.add_v(&-this.translation);
    field::T::density(&this.field, &p)
  }

  fn normal(this: &Self, p: &Point3<f32>) -> Vector3<f32> {
    let p = p.add_v(&-this.translation);
    field::T::normal(&this.field, &p)
  }
}
