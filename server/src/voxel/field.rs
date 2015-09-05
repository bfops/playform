use cgmath::{Point3, Vector3};

use voxel;

/// A trait representing a density field. The field does not need to be defined everywhere.
pub trait T {
  /// The density of the material at this point. This should be nonnegative.
  fn density_at(&Self, p: &Point3<f32>) -> f32;

  /// The material at this point.
  fn material_at(this: &Self, p: &Point3<f32>) -> Option<voxel::Material>;

  /// The surface normal at a given point.
  fn normal_at(this: &Self, delta: f32, p: &Point3<f32>) -> Vector3<f32>;
}
