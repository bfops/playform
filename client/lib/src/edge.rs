use cgmath;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction { X, Y, Z }

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct T {
  pub low_corner: cgmath::Point3<i32>,
  pub lg_size: i16,
  pub direction: Direction,
}
