use cgmath::{Point, Point3, Vector, EuclideanVector, Vector3};
use std::cmp::{min, max};
use std::ops::Neg;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Bounds {
  /// x-coordinate as a multiple of 2^lg_size.
  pub x: i32,
  /// y-coordinate as a multiple of 2^lg_size.
  pub y: i32,
  /// z-coordinate as a multiple of 2^lg_size.
  pub z: i32,
  /// The log_2 of the voxel's size.
  pub lg_size: i16,
}

impl Bounds {
  /// Convenience function to create `Bounds`.
  /// N.B. That the input coordinates should be divided by (2^lg_size) relative to world coords.
  pub fn new(x: i32, y: i32, z: i32, lg_size: i16) -> Bounds {
    let ret =
      Bounds {
        x: x,
        y: y,
        z: z,
        lg_size: lg_size,
      };
    ret
  }

  /// The width of this voxel.
  #[inline(always)]
  pub fn size(&self) -> f32 {
    if self.lg_size >= 0 {
      (1 << self.lg_size) as f32
    } else {
      1.0 / (1 << -self.lg_size) as f32
    }
  }

  pub fn low_corner(&self) -> Point3<f32> {
    let size = self.size();
    Point3::new(self.x as f32, self.y as f32, self.z as f32).mul_s(size)
  }

  pub fn corners(&self) -> (Point3<f32>, Point3<f32>) {
    let size = self.size();
    let low = Point3::new(self.x as f32, self.y as f32, self.z as f32).mul_s(size);
    (low, low.add_v(&Vector3::new(size, size, size)))
  }

  pub fn center(&self) -> Point3<f32> {
    let size = self.size();
    let half = Vector3::new(0.5, 0.5, 0.5);
    Point3::new(self.x as f32, self.y as f32, self.z as f32).add_v(&half).mul_s(size)
  }

  pub fn contains(&self, p: &Point3<f32>) -> bool {
    let (low, high) = self.corners();
    p.x >= low.x &&
    p.y >= low.y &&
    p.z >= low.z &&
    p.x < high.x &&
    p.y < high.y &&
    p.z < high.z &&
    true
  }
}

// NOTE: When voxel size and storage become an issue, this should be shrunk to
// be less than pointer-sized. It'll be easier to transfer to the GPU for
// whatever reasons, but also make it possible to shrink the SVO footprint by
// "flattening" the leaf contents and pointers into the same space (the
// low-order bits can be used to figure out which one it is, since pointers
// have three low-order bits set to zero).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Voxel {
  // The voxel is entirely inside or outside the volume. true is inside.
  Volume(bool),
  // The voxel crosses the surface of the volume.
  Surface(SurfaceVoxel),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
// Every voxel keeps track of a single vertex, as well as whether its
// lowest-coordinate corner is inside the volume.
// Since we keep track of an "arbitrarily" large world of voxels, we don't
// leave out any corners.
pub struct SurfaceVoxel {
  /// The position of a free-floating vertex on the surface.
  pub inner_vertex: Vertex,
  /// The surface normal at `inner_vertex`.
  pub normal: Normal,

  /// Is this voxel's low corner inside the field?
  pub corner_inside_surface: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Vertex {
  pub x: Fracu8,
  pub y: Fracu8,
  pub z: Fracu8,
}

impl Vertex {
  pub fn of_world_vertex_in(vertex: &Point3<f32>, voxel: &Bounds) -> Vertex {
    assert!(voxel.contains(vertex));

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

  pub fn to_world_vertex(&self, parent: &Bounds) -> Point3<f32> {
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

