//! A mosaic implementation defined by the union of other mosaics.

use cgmath::{Point3, Vector3};
use std::f32;

use field;
use mosaic;

#[allow(missing_docs)]
pub struct T<Material> {
  components: Vec<(Box<field::T>, Material)>,
}

unsafe impl<Material> Send for T<Material> {}

#[allow(missing_docs)]
pub fn new<Material>() -> T<Material> {
  T {
    components: Vec::new(),
  }
}

impl<Material> T<Material> {
  /// Add a component.
  pub fn push<Field>(&mut self, material: Material, field: Field)
    where Field: field::T + 'static,
  {
    self.components.push((Box::new(field), material));
  }
}

impl<Material> field::T for T<Material> {
  fn density(&self, p: &Point3<f32>) -> f32 {
    assert!(!self.components.is_empty());
    self.components.iter().fold(
      f32::NEG_INFINITY,
      |max, &(ref shape, _)| f32::max(max, shape.density(p)),
    )
  }

  fn normal(&self, p: &Point3<f32>) -> Vector3<f32> {
    assert!(!self.components.is_empty());
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

impl<Material> mosaic::T for T<Material> where Material: Eq + Clone {
  type Material = Material;

  fn material(&self, p: &Point3<f32>) -> Option<Material> {
    assert!(!self.components.is_empty());
    let (_, material) =
      self.components.iter().fold(
        (f32::NEG_INFINITY, None),
        |(max, max_material), &(ref shape, ref material)| {
          let d = shape.density(p);
          if d > max && d >= 0.0 {
            (d, Some(material.clone()))
          } else {
            (max, max_material)
          }
        },
      );
    material
  }
}
