use cgmath::{Point3};

use voxel;

pub mod solid;
pub mod tree;

/// A density field that also defines materials. This does not need to be defined everywhere.
pub trait T: voxel::field::T {
  /// The material density at a given point. This should be nonnegative!
  fn density(this: &Self, p: &Point3<f32>) -> f32 {
    voxel::field::T::density(this, p).abs()
  }

  /// The material at this point.
  fn material(this: &Self, p: &Point3<f32>) -> Option<voxel::Material>;
}