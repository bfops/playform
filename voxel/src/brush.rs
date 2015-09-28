//! Voxel brush module

use cgmath::Aabb3;

#[allow(missing_docs)]
pub type Bounds = Aabb3<i32>;

#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct T<Mosaic> {
  /// The bounds of this brush.
  pub bounds: Bounds,
  /// The mosaic that this brush will apply.
  pub mosaic: Mosaic,
}

unsafe impl<Mosaic> Send for T<Mosaic> {}
