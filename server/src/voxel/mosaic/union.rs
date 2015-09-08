use cgmath::{Point3, Vector3};
use std::f32;

use voxel::field;

pub struct T {
  pub components: Vec<(Box<field::T>, ::voxel::Material)>,
}

unsafe impl Send for T {}

pub fn new() -> T {
  T {
    components: Vec::new(),
  }
}

impl T {
  pub fn push<Field>(&mut self, material: ::voxel::Material, field: Field)
    where Field: field::T + 'static,
  {
    self.components.push((Box::new(field), material));
  }
}

impl field::T for T {
  fn density(&self, p: &Point3<f32>) -> f32 {
    assert!(self.components.len() > 0);
    self.components.iter().fold(
      f32::NEG_INFINITY,
      |max, &(ref shape, _)| f32::max(max, shape.density(p)),
    )
  }

  fn normal(&self, p: &Point3<f32>) -> Vector3<f32> {
    assert!(self.components.len() > 0);
    let (_, normal) =
      self.components.iter().fold(
        (f32::NEG_INFINITY, Vector3::new(0.0, 0.0, 0.0)),
        |(max, normal), &(ref shape, _)| {
          let d = shape.density(p);
          if d > max {
            (d, shape.normal(p))
          } else {
            (max, normal)
          }
        },
      );
    normal
  }
}

impl ::voxel::mosaic::T for T {
  fn material(&self, p: &Point3<f32>) -> Option<::voxel::Material> {
    assert!(self.components.len() > 0);
    let (_, material) =
      self.components.iter().fold(
        (f32::NEG_INFINITY, None),
        |(max, max_material), &(ref shape, material)| {
          let d = shape.density(p);
          if d > max && d >= 0.0 {
            (d, Some(material))
          } else {
            (max, max_material)
          }
        },
      );
    material
  }
}
