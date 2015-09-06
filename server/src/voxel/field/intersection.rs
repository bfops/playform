use cgmath::{Point3, Vector3};

use voxel::field;

pub struct T ([Box<field::Dispatch>; 2]);

unsafe impl Send for T {}

pub fn new<Field1, Field2>(field1: Field1, field2: Field2) -> T
  where 
    Field1: field::T + 'static,
    Field2: field::T + 'static,
{
  T([Box::new(field1), Box::new(field2)])
}

impl field::T for T {
  fn density(t: &T, p: &Point3<f32>) -> f32 {
    f32::min(t.0[0].density(p), t.0[1].density(p))
  }

  fn normal(t: &T, p: &Point3<f32>) -> Vector3<f32> {
    let d1 = t.0[0].density(p);
    let d2 = t.0[1].density(p);
    if d1 < d2 {
      t.0[0].normal(p)
    } else {
      t.0[1].normal(p)
    }
  }
}
