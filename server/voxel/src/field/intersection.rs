//! A field defined by the intersection of a pair of fields.

use cgmath::{Point3, Vector3};

use field;

#[allow(missing_docs)]
pub struct T ([Box<field::T>; 2]);

unsafe impl Send for T {}

#[allow(missing_docs)]
pub fn new<Field1, Field2>(field1: Field1, field2: Field2) -> T
  where
    Field1: field::T + 'static,
    Field2: field::T + 'static,
{
  T([Box::new(field1), Box::new(field2)])
}

impl field::T for T {
  fn density(&self, p: &Point3<f32>) -> f32 {
    f32::min(self.0[0].density(p), self.0[1].density(p))
  }

  fn normal(&self, p: &Point3<f32>) -> Vector3<f32> {
    let d1 = self.0[0].density(p);
    let d2 = self.0[1].density(p);
    if d1 < d2 {
      self.0[0].normal(p)
    } else {
      self.0[1].normal(p)
    }
  }
}
