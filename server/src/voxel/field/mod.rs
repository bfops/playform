use cgmath::{Point3, Vector3};

pub mod sphere;
pub mod intersection;
pub mod translation;

/// A trait representing a density field.
pub trait T {
  /// The density of the material at this point.
  fn density(&self, p: &Point3<f32>) -> f32;

  /// The surface normal at a given point.
  fn normal(&self, p: &Point3<f32>) -> Vector3<f32>;
}
