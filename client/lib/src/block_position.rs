//! Position data structure for terrain blocks.

use cgmath::{Point3, Vector3, Point};
use std::ops::Add;

use edge;
use lod;
use terrain_mesh;

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
/// Position of blocks on an "infinite" regular grid.
/// The position is implicitly in units of terrain_mesh::WIDTH.
pub struct T(Point3<i32>);

pub struct Edges {
  lower: Point3<i32>,
  upper: Point3<i32>,
  cur: Point3<i32>,
  direction: edge::Direction,
  lg_size: i16,
}

impl Iterator for Edges {
  type Item = edge::T;

  fn next(&mut self) -> Option<Self::Item> {
    match self.direction {
      edge::Direction::X => {
        self.direction = edge::Direction::Y;
      },
      edge::Direction::Y => {
        self.direction = edge::Direction::Z;
      },
      edge::Direction::Z => {
        if self.cur.z < self.upper.z {
          self.cur.z += 1;
        } else {
          self.cur.z = self.lower.z;
          if self.cur.y < self.upper.y {
            self.cur.y += 1;
          } else {
            self.cur.y = self.lower.y;
            if self.cur.x < self.upper.x {
              self.cur.x += 1;
            } else {
              self.cur.x = self.lower.x;
              return None
            }
          }
        }
      },
    }

    Some(edge::T {
      low_corner: self.cur,
      direction: self.direction,
      lg_size: self.lg_size,
    })
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

  #[allow(missing_docs)]
  #[allow(dead_code)]
  pub fn to_world_position(&self) -> Point3<f32> {
    Point3::new(
      (self.as_pnt().x * terrain_mesh::WIDTH) as f32,
      (self.as_pnt().y * terrain_mesh::WIDTH) as f32,
      (self.as_pnt().z * terrain_mesh::WIDTH) as f32,
    )
  }

  pub fn edges(&self, lod: lod::T) -> Edges {
    let lg_edge_samples = terrain_mesh::LG_EDGE_SAMPLES[lod.0 as usize];
    let lg_sample_size = terrain_mesh::LG_SAMPLE_SIZE[lod.0 as usize];

    let low = self.as_pnt().clone();
    let high = low.add_v(&Vector3::new(1, 1, 1));
    let low =
      Point3::new(
        low.x << lg_edge_samples,
        low.y << lg_edge_samples,
        low.z << lg_edge_samples,
      );
    let high =
      Point3::new(
        high.x << lg_edge_samples,
        high.y << lg_edge_samples,
        high.z << lg_edge_samples,
      );

    Edges {
      lower: low,
      upper: high.add_v(&Vector3::new(-1, -1, -1)),
      cur: low,
      lg_size: lg_sample_size,
      direction: edge::Direction::X,
    }
  }
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
        x - (terrain_mesh::WIDTH - 1)
      } else {
        x
      };
    x >> terrain_mesh::LG_WIDTH
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
