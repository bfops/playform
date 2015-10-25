//! Position data structure for terrain blocks.

use cgmath::{Point3, Vector3};
use std::ops::Add;

use terrain_block;

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, RustcEncodable, RustcDecodable)]
/// Position of blocks on an "infinite" regular grid.
/// The position is implicitly in units of terrain_block::WIDTH.
pub struct BlockPosition(Point3<i32>);

impl BlockPosition {
  #[allow(missing_docs)]
  pub fn new(x: i32, y: i32, z: i32) -> BlockPosition {
    BlockPosition(Point3::new(x, y, z))
  }

  #[allow(missing_docs)]
  pub fn of_pnt(p: &Point3<i32>) -> BlockPosition {
    BlockPosition(p.clone())
  }

  #[allow(missing_docs)]
  pub fn as_pnt(&self) -> &Point3<i32> {
    let BlockPosition(ref pnt) = *self;
    pnt
  }

  #[allow(missing_docs)]
  pub fn as_mut_pnt(&mut self) -> &mut Point3<i32> {
    let BlockPosition(ref mut pnt) = *self;
    pnt
  }

  #[allow(missing_docs)]
  pub fn of_world_position(world_position: &Point3<f32>) -> BlockPosition {
    macro_rules! convert_coordinate(
      ($x: expr) => ({
        let x = $x.floor() as i32;
        let x =
          if x < 0 {
            x - (terrain_block::WIDTH - 1)
          } else {
            x
          };
        x / terrain_block::WIDTH
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
      (self.as_pnt().x * terrain_block::WIDTH) as f32,
      (self.as_pnt().y * terrain_block::WIDTH) as f32,
      (self.as_pnt().z * terrain_block::WIDTH) as f32,
    )
  }
}

impl Add<Vector3<i32>> for BlockPosition {
  type Output = BlockPosition;

  fn add(mut self, rhs: Vector3<i32>) -> Self {
    self.as_mut_pnt().x += rhs.x;
    self.as_mut_pnt().y += rhs.y;
    self.as_mut_pnt().z += rhs.z;
    self
  }
}
