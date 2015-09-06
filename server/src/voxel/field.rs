use cgmath::{Point3, Vector3};

/// A trait representing a density field.
pub trait T {
  /// The density of the material at this point. This should be nonnegative.
  fn density(&Self, p: &Point3<f32>) -> f32;

  /// The surface normal at a given point.
  fn normal(this: &Self, delta: f32, p: &Point3<f32>) -> Vector3<f32>;
}
