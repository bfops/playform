use cgmath::{Point3, Vector3, EuclideanVector};

mod voxel {
  pub use super::super::super::*;
}

#[derive(Debug, Clone, Copy)]
pub struct T {
  pub low: Point3<f32>,
  pub high: Point3<f32>,
}

unsafe impl Send for T {}

impl ::voxel::field::T for T {
  fn density_at(this: &Self, p: &Point3<f32>) -> f32 {
    (p.x - this.low.x) * (this.high.x - p.x) *
    (p.y - this.low.y) * (this.high.y - p.y) *
    (p.z - this.low.z) * (this.high.z - p.z) *
    1.0
  }

  fn contains(this: &Self, p: &Point3<f32>) -> bool {
    this.low.x <= p.x && p.x <= this.high.x &&
    this.low.y <= p.y && p.y <= this.high.y &&
    this.low.z <= p.z && p.z <= this.high.z &&
    true
  }

  fn normal_at(_: &Self, _: f32, _: &Point3<f32>) -> Vector3<f32> {
    unimplemented!();
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

impl voxel::brush::T for T {
  fn intersect(this: &Self, voxel: &::voxel::Bounds) -> voxel::brush::Intersection {
    let (low, high) = voxel.corners();

    let mut touches_surface = false;

    macro_rules! in1D(($d:ident) => {{
      match SegmentOverlap::of(low.$d, high.$d, this.low.$d, this.high.$d) {
        SegmentOverlap::None => {
          return voxel::brush::Intersection::Outside
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
          if surface_point == this.low.$d {
            (surface_point, -1.0)
          } else if surface_point == this.high.$d {
            (surface_point, 1.0)
          } else {
            panic!("SegmentOverlap::Partial matches no inputs");
          }
        },
      }
    }});

    let (x, nx) = in1D!(x);
    let (y, ny) = in1D!(y);
    let (z, nz) = in1D!(z);

    if !touches_surface {
      return super::Intersection::Inside;
    }

    let vtx = Point3::new(x, y, z);
    let normal = Vector3::new(nx, ny, nz);

    let vtx = voxel::Vertex::of_world_vertex_in(&vtx, voxel);
    let normal = voxel::Normal::of_float_normal(&normal.normalize());

    super::Intersection::Crosses(vtx, normal)
  }
}
