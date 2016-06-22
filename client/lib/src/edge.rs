use cgmath::{Point3, Vector3, Point};

use voxel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction { X, Y, Z }

impl Direction {
  pub fn to_vec(self) -> Vector3<i32> {
    match self {
      Direction::X => Vector3::new(1, 0, 0),
      Direction::Y => Vector3::new(0, 1, 0),
      Direction::Z => Vector3::new(0, 0, 1),
    }
  }

  pub fn perpendicular(self) -> (Direction, Direction) {
    match self {
      Direction::X => (Direction::Y, Direction::Z),
      Direction::Y => (Direction::Z, Direction::X),
      Direction::Z => (Direction::X, Direction::Y),
    }
  }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct T {
  pub low_corner: Point3<i32>,
  pub lg_size: i16,
  pub direction: Direction,
}

impl T {
  pub fn neighbors(&self) -> [voxel::bounds::T; 4] {
    let (v1, v2): (Direction, Direction) = self.direction.perpendicular();
    let (v1, v2) = (-v1.to_vec(), -v2.to_vec());
    let make_bounds = |p: Point3<i32>| voxel::bounds::new(p.x, p.y, p.z, self.lg_size);
    [
      make_bounds(self.low_corner),
      make_bounds(self.low_corner.add_v(&v1)),
      make_bounds(self.low_corner.add_v(&v1).add_v(&v2)),
      make_bounds(self.low_corner.add_v(&v2)),
    ]
  }
}

pub mod set {
  use common::fnv_set;

  pub type T = fnv_set::T<super::T>;

  #[allow(dead_code)]
  pub fn new() -> T {
    fnv_set::new()
  }
}
