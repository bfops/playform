use cgmath::Point3;

use super::super::field::Field;
use super::super::voxel;

pub trait Brush: Field {
  fn vertex_in(&self, bounds: &voxel::Bounds) -> Option<(voxel::Vertex, voxel::Normal)>;
}

#[derive(Debug)]
pub struct Bounds {
  pub low: Point3<i32>,
  pub high: Point3<i32>,
}
