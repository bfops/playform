use cgmath;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction { X, Y, Z }

impl Direction {
  pub fn to_vec(self) -> cgmath::Vector3<i32> {
    match self {
      Direction::X => cgmath::Vector3::new(1, 0, 0),
      Direction::Y => cgmath::Vector3::new(0, 1, 0),
      Direction::Z => cgmath::Vector3::new(0, 0, 1),
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
  pub low_corner: cgmath::Point3<i32>,
  pub lg_size: i16,
  pub direction: Direction,
}
