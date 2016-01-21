use cgmath::{Aabb, Point3, Vector3};

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct T {
  pub low_corner: Point3<i32>,
  pub lg_size: i16,
  pub direction: Direction,
}

pub mod set {
  use fnv::FnvHasher;
  use std;

  pub type T = std::collections::HashSet<super::T, std::hash::BuildHasherDefault<FnvHasher>>;

  #[allow(dead_code)]
  pub fn new() -> T {
    std::collections::HashSet::with_hasher(Default::default())
  }
}

pub mod map {
  use fnv::FnvHasher;
  use std;

  pub type T<V> = std::collections::HashMap<super::T, V, std::hash::BuildHasherDefault<FnvHasher>>;

  pub fn new<V>() -> T<V> {
    std::collections::HashMap::with_hasher(Default::default())
  }
}
