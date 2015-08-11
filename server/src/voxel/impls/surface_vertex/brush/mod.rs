use cgmath::{Point3};

use voxel;

pub mod cube;
pub mod sphere;

/// Fields that can act as voxel brushes.
pub trait T: voxel::field::T {
  /// Get a "representative" vertex for some voxel.
  fn vertex_in(&Self, bounds: &voxel::Bounds) -> Option<(voxel::Vertex, voxel::Normal)>;
}

/// The (approximate) area covered by the brush.
#[derive(Debug)]
pub struct Bounds {
  pub low: Point3<i32>,
  pub high: Point3<i32>,
}
