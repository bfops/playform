use cgmath::{Point, Point3, EuclideanVector, Vector3};

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
  /// N.B. That the input coordinates should be divided by (2^lg_size).
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
}

// NOTE: When voxel size and storage become an issue, this should be shrunk to
// be less than pointer-sized. It'll be easier to transfer to the GPU for
// whatever reasons, but also make it possible to shrink the SVO footprint by
// "flattening" the leaf contents and pointers into the same space (the
// low-order bits can be used to figure out which one it is, since pointers
// have three low-order bits set to zero).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Voxel {
  Empty,
  Surface(SurfaceVoxel),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SurfaceVoxel {
  pub vertex: VoxelVertex,
  pub normal: VoxelNormal,

  // Each voxel contains edge information about the edges that intersect the
  // lowest corner of the cube. Assuming the grid is effectively infinite, no
  // edges will be left out.

  pub x_edge: Edge,
  pub y_edge: Edge,
  pub z_edge: Edge,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct VoxelVertex {
  pub x: Fracu8,
  pub y: Fracu8,
  pub z: Fracu8,
}

impl VoxelVertex {
  pub fn to_world_vertex(&self, parent: Bounds) -> Point3<f32> {
    // Relative position of the vertex.
    let local =
      Vector3::new(
        self.x.numerator as f32 / 256.0,
        self.y.numerator as f32 / 256.0,
        self.z.numerator as f32 / 256.0,
      );
    let fparent = Point3::new(parent.x as f32, parent.y as f32, parent.z as f32);
    fparent.add_v(&local).mul_s(parent.size())
  }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct VoxelNormal {
  pub x: Fraci8,
  pub y: Fraci8,
  pub z: Fraci8,
}

impl VoxelNormal {
  pub fn to_world_normal(&self) -> Vector3<f32> {
    Vector3::new(self.x.to_f32(), self.y.to_f32(), self.z.to_f32()).normalize()
  }
}

/// Tells you whether and where the surface crossed an edge of a cubic voxel.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Edge {
  pub is_crossed: bool,
  // If this is true, the edge moves into the volume as its coordinates increase.
  pub direction: bool,
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

/// Express a `[0,1)` fraction using a `i8`.
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

