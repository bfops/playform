//! Position data structure for terrain blocks.

/// lg(WIDTH)
pub const LG_WIDTH: u16 = 3;
/// The width of a chunk of terrain.
pub const WIDTH: u32 = 1 << LG_WIDTH;

#[allow(missing_docs)]
pub mod position {
  use cgmath::{Point3, Vector3};
  use std::ops::Add;

  use common::voxel;

  use chunk;

  #[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
  /// Position of blocks on an "infinite" regular grid.
  /// The position is implicitly in units of chunk::WIDTH.
  pub struct T(Point3<i32>);

  /// chunk position that contains a given voxel
  #[inline(never)]
  pub fn containing_voxel(bounds: &voxel::bounds::T) -> T {
    if bounds.lg_size < 0 {
      new(
        (bounds.x >> -bounds.lg_size) >> chunk::LG_WIDTH,
        (bounds.y >> -bounds.lg_size) >> chunk::LG_WIDTH,
        (bounds.z >> -bounds.lg_size) >> chunk::LG_WIDTH,
      )
    } else {
      new(
        (bounds.x << bounds.lg_size) >> chunk::LG_WIDTH,
        (bounds.y << bounds.lg_size) >> chunk::LG_WIDTH,
        (bounds.z << bounds.lg_size) >> chunk::LG_WIDTH,
      )
    }
  }

  impl T {
    #[allow(missing_docs)]
    pub fn as_pnt(&self) -> &Point3<i32> {
      let T(ref pnt) = *self;
      pnt
    }

    #[allow(missing_docs)]
    pub fn as_mut_pnt(&mut self) -> &mut Point3<i32> {
      let T(ref mut pnt) = *self;
      pnt
    }
  }

  #[allow(missing_docs)]
  pub fn of_pnt(p: &Point3<i32>) -> T {
    T(*p)
  }

  #[allow(missing_docs)]
  pub fn new(x: i32, y: i32, z: i32) -> T {
    T(Point3::new(x, y, z))
  }

  #[allow(missing_docs)]
  pub fn of_world_position(world_position: &Point3<f32>) -> T {
    fn convert_coordinate(x: f32) -> i32 {
      let x = x.floor() as i32;
      let x =
        if x < 0 {
          x - (chunk::WIDTH as i32 - 1)
        } else {
          x
        };
      x >> chunk::LG_WIDTH
    }

    T(
      Point3::new(
        convert_coordinate(world_position.x),
        convert_coordinate(world_position.y),
        convert_coordinate(world_position.z),
      )
    )
  }

  impl Add<Vector3<i32>> for T {
    type Output = T;

    fn add(mut self, rhs: Vector3<i32>) -> Self {
      self.as_mut_pnt().x += rhs.x;
      self.as_mut_pnt().y += rhs.y;
      self.as_mut_pnt().z += rhs.z;
      self
    }
  }
}
