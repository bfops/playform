use cgmath::{Point, Point3, Vector, EuclideanVector, Vector3};
use std::cmp::{min, max};
use std::ops::Neg;

use voxel;

pub mod brush;

// NOTE: When voxel size and storage become an issue, this should be shrunk to
// be less than pointer-sized. It'll be easier to transfer to the GPU for
// whatever reasons, but also make it possible to shrink the SVO footprint by
// "flattening" the leaf contents and pointers into the same space (the
// low-order bits can be used to figure out which one it is, since pointers
// have three low-order bits set to zero).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum T {
  // The voxel is entirely inside or outside the volume. true is inside.
  Volume(bool),
  // The voxel crosses the surface of the volume.
  Surface(SurfaceStruct),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
// Every voxel keeps track of a single vertex, as well as whether its
// lowest-coordinate corner is inside the volume.
// Since we keep track of an "arbitrarily" large world of voxels, we don't
// leave out any corners.
pub struct SurfaceStruct {
  /// The position of a free-floating vertex on the surface.
  pub inner_vertex: Vertex,
  /// The surface normal at `inner_vertex`.
  pub normal: Normal,

  /// Is this voxel's low corner inside the field?
  pub corner_inside_surface: bool,
}

impl<Brush> voxel::brush::T for Brush where Brush: brush::T {
  type Voxel = T;

  fn remove(
    this: &mut T,
    bounds: &voxel::Bounds,
    brush: &Brush,
  ) {
    let set_leaf = |this: &mut T, corner_inside_surface| {
      match brush::T::intersect(brush, bounds) {
        brush::Intersection::Outside => {},
        brush::Intersection::Inside => {
          *this = T::Volume(false);
        },
        brush::Intersection::Crosses(vertex, normal) => {
          let size = bounds.size();
          let low = Point3::new(bounds.x as f32, bounds.y as f32, bounds.z as f32);
          let low = low.mul_s(size);
          // The brush is negative space, so the normal should point into it, not out of it.
          let normal = -normal;
          let corner_inside_surface = corner_inside_surface && !voxel::field::T::contains(brush, &low);
          let voxel =
            SurfaceStruct {
              inner_vertex: vertex,
              normal: normal,
              corner_inside_surface: corner_inside_surface,
            };
          *this = T::Surface(voxel);
        },
      }
    };

    match this {
      &mut T::Volume(false) => {},
      &mut T::Volume(true) => {
        set_leaf(this, true);
      },
      &mut T::Surface(voxel) => {
        set_leaf(this, voxel.corner_inside_surface);
      },
    }
  }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Vertex {
  pub x: Fracu8,
  pub y: Fracu8,
  pub z: Fracu8,
}

impl Vertex {
  pub fn of_world_vertex_in(vertex: &Point3<f32>, voxel: &voxel::Bounds) -> Vertex {
    assert!(voxel.contains(vertex), "{:?} does not contain {:?}", voxel, vertex);

    let local = vertex.sub_p(&voxel.low_corner());
    let local = local.mul_s(256.0);
    let local =
      Vector3::new(
        local.x as u32 >> voxel.lg_size,
        local.y as u32 >> voxel.lg_size,
        local.z as u32 >> voxel.lg_size,
      );
    Vertex {
      x: Fracu8::of(min(0, max(255, local.x as u8))),
      y: Fracu8::of(min(0, max(255, local.y as u8))),
      z: Fracu8::of(min(0, max(255, local.z as u8))),
    }
  }

  pub fn to_world_vertex(&self, parent: &voxel::Bounds) -> Point3<f32> {
    // Relative position of the vertex.
    let local =
      Vector3::new(
        self.x.numerator as f32,
        self.y.numerator as f32,
        self.z.numerator as f32,
      )
      .div_s(256.0)
    ;
    let fparent = Point3::new(parent.x as f32, parent.y as f32, parent.z as f32);
    fparent.add_v(&local).mul_s(parent.size())
  }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Normal {
  pub x: Fraci8,
  pub y: Fraci8,
  pub z: Fraci8,
}

impl Normal {
  pub fn of_float_normal(normal: &Vector3<f32>) -> Normal {
    // Okay, so we scale the normal by 127, and use 127 to represent 1.0.
    // Then we store it in a `Fraci8`, which scales by 128 and represents a
    // fraction in [-1,1). That seems wrong, but this is normal data, so scaling
    // doesn't matter. Sketch factor is over 9000, but it's not wrong.

    let normal = normal.mul_s(127.0);
    let normal = Vector3::new(normal.x as i32, normal.y as i32, normal.z as i32);
    Normal {
      x: Fraci8::of(max(-127, min(127, normal.x)) as i8),
      y: Fraci8::of(max(-127, min(127, normal.y)) as i8),
      z: Fraci8::of(max(-127, min(127, normal.z)) as i8),
    }
  }

  pub fn to_float_normal(&self) -> Vector3<f32> {
    Vector3::new(self.x.to_f32(), self.y.to_f32(), self.z.to_f32()).normalize()
  }
}

impl Neg for Normal {
  type Output = Normal;

  fn neg(self) -> Normal {
    Normal {
      x: Fraci8::of(-max(-127, self.x.numerator)),
      y: Fraci8::of(-max(-127, self.y.numerator)),
      z: Fraci8::of(-max(-127, self.z.numerator)),
    }
  }
}

/// Express a `[0,1)` fraction using a `u8`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Fracu8 {
  // The denominator is 1 << 8.
  pub numerator: u8,
}

impl Fracu8 {
  pub fn of(numerator: u8) -> Fracu8 {
    Fracu8 {
      numerator: numerator,
    }
  }
}

/// Express a `[-1,1)` fraction using a `i8`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Fraci8 {
  // The denominator is 1 << 8.
  pub numerator: i8,
}

impl Fraci8 {
  pub fn of(numerator: i8) -> Fraci8 {
    Fraci8 {
      numerator: numerator,
    }
  }

  pub fn to_f32(&self) -> f32 {
    self.numerator as f32 / 128.0
  }
}
