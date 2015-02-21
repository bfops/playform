//! Position data structure for terrain blocks.

use cgmath::{Point3, Vector3};
use std::num::Float;
use std::ops::Add;

pub const BLOCK_WIDTH: i32 = 8;

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
#[derive(RustcDecodable, RustcEncodable)]
/// Position of blocks on an "infinite" regular grid.
pub struct BlockPosition(Point3<i32>);

impl BlockPosition {
  #[inline(always)]
  #[allow(missing_docs)]
  pub fn new(x: i32, y: i32, z: i32) -> BlockPosition {
    BlockPosition(Point3::new(x, y, z))
  }

  #[inline(always)]
  #[allow(missing_docs)]
  pub fn as_pnt<'a>(&'a self) -> &'a Point3<i32> {
    let BlockPosition(ref pnt) = *self;
    pnt
  }

  #[inline(always)]
  #[allow(missing_docs)]
  pub fn as_mut_pnt3<'a>(&'a mut self) -> &'a mut Point3<i32> {
    let BlockPosition(ref mut pnt) = *self;
    pnt
  }

  #[allow(missing_docs)]
  pub fn from_world_position(world_position: &Point3<f32>) -> BlockPosition {
    macro_rules! convert_coordinate(
      ($x: expr) => ({
        let x = $x.floor() as i32;
        let x =
          if x < 0 {
            x - (BLOCK_WIDTH - 1)
          } else {
            x
          };
        x / BLOCK_WIDTH
      })
    );
    BlockPosition(
      Point3::new(
        convert_coordinate!(world_position.x),
        convert_coordinate!(world_position.y),
        convert_coordinate!(world_position.z),
      )
    )
  }

  #[allow(missing_docs)]
  pub fn to_world_position(&self) -> Point3<f32> {
    Point3::new(
      (self.as_pnt().x * BLOCK_WIDTH) as f32,
      (self.as_pnt().y * BLOCK_WIDTH) as f32,
      (self.as_pnt().z * BLOCK_WIDTH) as f32,
    )
  }
}

impl Add<Vector3<i32>> for BlockPosition {
  type Output = BlockPosition;

  fn add(mut self, rhs: Vector3<i32>) -> Self {
    self.as_mut_pnt3().x += rhs.x;
    self.as_mut_pnt3().y += rhs.y;
    self.as_mut_pnt3().z += rhs.z;
    self
  }
}
