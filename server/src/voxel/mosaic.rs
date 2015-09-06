use cgmath::{Point3};

use voxel;

/// A density field that also defines materials. This does not need to be defined everywhere.
pub trait T: voxel::field::T {
  /// The material at this point.
  fn material(this: &Self, p: &Point3<f32>) -> Option<voxel::Material>;
}
