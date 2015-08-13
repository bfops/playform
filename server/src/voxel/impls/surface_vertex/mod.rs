use cgmath::{Point, Point3, Vector};

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
  pub inner_vertex: voxel::Vertex,
  /// The surface normal at `inner_vertex`.
  pub normal: voxel::Normal,

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
      match brush::T::vertex_in(brush, bounds) {
        None => {},
        Some((vertex, normal)) => {
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
