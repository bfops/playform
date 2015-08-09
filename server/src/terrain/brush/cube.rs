use cgmath::{Point3, Vector3, EuclideanVector};

use super::super::field::Field;
use super::super::voxel;

use super::types::Brush;
use super::types::Bounds;

#[derive(Debug, Clone, Copy)]
pub struct Cube {
  pub low: Point3<f32>,
  pub high: Point3<f32>,
}

unsafe impl Send for Cube {}

impl Field for Cube {
  fn density_at(&self, p: &Point3<f32>) -> f32 {
    (p.x - self.low.x) * (self.high.x - p.x) *
    (p.y - self.low.y) * (self.high.y - p.y) *
    (p.z - self.low.z) * (self.high.z - p.z) *
    1.0
  }

  fn contains(&self, p: &Point3<f32>) -> bool {
    self.low.x <= p.x && p.x <= self.high.x &&
    self.low.y <= p.y && p.y <= self.high.y &&
    self.low.z <= p.z && p.z <= self.high.z &&
    true
  }

  fn normal_at(&self, _: f32, _: &Point3<f32>) -> Vector3<f32> {
    panic!("Cube::normal_at");
  }
}

/// Whether a line segment contains another segment.
enum SegmentOverlap {
  FirstContainsSecond,
  SecondContainsFirst,
  /// The segments each contain one of the other's ends.
  Partial(f32, f32),
  /// The segments do not overlap.
  None,
}

impl SegmentOverlap {
  pub fn of(x00: f32, x01: f32, x10: f32, x11: f32) -> SegmentOverlap {
    assert!(x00 < x01);
    assert!(x10 < x11);

    if x00 <= x10 {
      if x01 < x11 {
        if x01 <= x10 {
          SegmentOverlap::None
        } else {
          SegmentOverlap::Partial(x01, x10)
        }
      } else {
        SegmentOverlap::FirstContainsSecond
      }
    } else {
      if x01 < x11 {
        SegmentOverlap::SecondContainsFirst
      } else {
        if x11 <= x00 {
          SegmentOverlap::None
        } else {
          SegmentOverlap::Partial(x00, x11)
        }
      }
    }
  }
}

impl Brush for Cube {
  fn vertex_in(&self, voxel: &voxel::Bounds) -> Option<(voxel::Vertex, voxel::Normal)> {
    let (low, high) = voxel.corners();

    let mut touches_surface = false;

    macro_rules! in1D(($d:ident) => {{
      match SegmentOverlap::of(low.$d, high.$d, self.low.$d, self.high.$d) {
        SegmentOverlap::None => {
          return None
        },
        SegmentOverlap::FirstContainsSecond => {
          panic!("FirstContainsSecond");
        },
        SegmentOverlap::SecondContainsFirst => {
          // Put a point in the middle of the voxel.
          ((low.$d + high.$d) / 2.0, 0.0)
        },
        SegmentOverlap::Partial(_, surface_point) => {
          touches_surface = true;
          if surface_point == self.low.$d {
            (surface_point, 1.0)
          } else {
            (surface_point, -1.0)
          }
        },
      }
    }});

    let (x, nx) = in1D!(x);
    let (y, ny) = in1D!(y);
    let (z, nz) = in1D!(z);

    if !touches_surface {
      return None;
    }

    let vtx = voxel::Vertex::of_world_vertex_in(&Point3::new(x, y, z), voxel);
    let normal = voxel::Normal::of_float_normal(&Vector3::new(nx, ny, nz).normalize());

    Some((vtx, normal))
  }
}
