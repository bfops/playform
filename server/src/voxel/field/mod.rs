use cgmath::{Point3, Vector3};

pub mod sphere;

/// A trait representing a density field.
pub trait T {
  /// The density of the material at this point.
  fn density(&Self, p: &Point3<f32>) -> f32;

  /// The surface normal at a given point.
  fn normal(this: &Self, p: &Point3<f32>) -> Vector3<f32>;
}
