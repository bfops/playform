pub use common::chunk::{T, LG_WIDTH, WIDTH};

pub mod position {
  use cgmath;

  use chunk;
  use edge;
  use voxel;

  pub use common::chunk::position::*;

  #[allow(missing_docs)]
  pub fn of_world_position(world_position: &cgmath::Point3<f32>) -> T {
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

    T {
      as_point:
        cgmath::Point3::new(
          convert_coordinate(world_position.x),
          convert_coordinate(world_position.y),
          convert_coordinate(world_position.z),
        ),
    }
  }

  pub fn containing(voxel: &voxel::bounds::T) -> T {
    let lg_ratio = chunk::LG_WIDTH as i16 - voxel.lg_size;
    assert!(lg_ratio >= 0);
    T {
      as_point :
        cgmath::Point3::new(
          voxel.x >> lg_ratio,
          voxel.y >> lg_ratio,
          voxel.z >> lg_ratio,
        ),
    }
  }

  pub struct InnerEdges {
    chunk     : T,
    lg_size   : i16,
    width     : u32,
    current   : cgmath::Vector3<i32>,
    direction : edge::Direction,
    done      : bool,
  }

  impl InnerEdges {
    #[allow(missing_docs)]
    pub fn new(chunk: T, lg_size: i16) -> Self {
      let width = 1 << (chunk::LG_WIDTH as u32 - lg_size as u32);
      InnerEdges {
        chunk     : chunk,
        lg_size   : lg_size,
        width     : width,
        current   : cgmath::Vector3::new(0, 1, 1),
        direction : edge::Direction::X,
        done      : width <= 1,
      }
    }
  }

  impl Iterator for InnerEdges {
    type Item = edge::T;
    fn next(&mut self) -> Option<Self::Item> {
      if self.done {
        return None
      }

      macro_rules! load(
        ($x:ident, $y:ident, $z:ident) => {{
          let r =
            Some(edge::T {
              low_corner : (self.chunk.as_point * self.width as i32 + self.current),
              lg_size    : self.lg_size,
              direction  : self.direction,
            });

          self.current.$z += 1;
          if (self.current.$z as u32) <= self.width - 1 { return r }
          self.current.$z = 1;

          self.current.$y += 1;
          if (self.current.$y as u32) <= self.width - 1 { return r }
          self.current.$y = 1;

          self.current.$x += 1;
          if (self.current.$x as u32) <= self.width - 2 { return r }

          r
        }}
      );

      match self.direction {
        edge::Direction::X => {
          let r = load!(x, y, z);
          self.direction = edge::Direction::Y;
          self.current = cgmath::Vector3::new(1, 0, 1);
          r
        },
        edge::Direction::Y => {
          let r = load!(y, z, x);
          self.direction = edge::Direction::Z;
          self.current = cgmath::Vector3::new(1, 1, 0);
          r
        },
        edge::Direction::Z => {
          let r = load!(z, x, y);
          self.done = true;
          r
        },
      }
    }
  }

  // the code for this is absolutely horrifying linguini, but I think it can
  // all disappear once we get `impl Trait` in returns.
  pub enum FaceEdges {
    Done,
    Perpendicular {
      chunk     : T,
      direction : edge::Direction,
      lg_size   : i16,
      width     : u32,
      current   : cgmath::Vector3<i32>,
    },
    Parallel1 {
      chunk     : T,
      direction : edge::Direction,
      lg_size   : i16,
      width     : u32,
      current   : cgmath::Vector3<i32>,
    },
    Parallel2 {
      chunk     : T,
      direction : edge::Direction,
      lg_size   : i16,
      width     : u32,
      current   : cgmath::Vector3<i32>,
    },
  }

  impl FaceEdges {
    pub fn new(direction: edge::Direction, chunk: T, lg_size: i16) -> Self {
      let width: u32 = 1 << (chunk::LG_WIDTH as i16 - lg_size);
      if width <= 1 {
        FaceEdges::Done
      } else {
        FaceEdges::Perpendicular {
          chunk     : chunk,
          direction : direction,
          lg_size   : lg_size,
          width     : width,
          current   : cgmath::Vector3::new(width as i32 - 1, 1, 1),
        }
      }
    }
  }

  impl Iterator for FaceEdges {
    type Item = edge::T;
    fn next(&mut self) -> Option<Self::Item> {
      let mut next = None;
      let r;
      match self {
        &mut FaceEdges::Done => return None,
        &mut FaceEdges::Perpendicular { chunk, direction, lg_size, width, ref mut current } => {
          let (x, (y, z)) = (direction, direction.perpendicular());
          let (xv, yv, zv) = (x.to_vec(), y.to_vec(), z.to_vec());
          let base = (chunk - x.to_vec()).as_point * width as i32;
          r =
            edge::T {
              low_corner : base + xv*current.x + yv*current.y + zv*current.z,
              direction  : direction,
              lg_size    : lg_size,
            };
          current.z += 1;
          if current.z > width as i32 - 1 {
            current.z = 1;
            current.y += 1;
            if current.y > width as i32 - 1 {
              next =
                Some(
                  FaceEdges::Parallel1 {
                    chunk     : chunk,
                    direction : direction,
                    lg_size   : lg_size,
                    width     : width,
                    current   : cgmath::Vector3::new(0, 0, 1),
                  }
                );
            }
          }
        },
        &mut FaceEdges::Parallel1 { chunk, direction, lg_size, width, ref mut current } => {
          let (x, (y, z)) = (direction, direction.perpendicular());
          let (xv, yv, zv) = (x.to_vec(), y.to_vec(), z.to_vec());
          let base = chunk.as_point * width as i32;
          r =
            edge::T {
              low_corner : base + xv*current.x + yv*current.y + zv*current.z,
              direction  : y,
              lg_size    : lg_size,
            };
          current.z += 1;
          if current.z > width as i32 - 1 {
            current.z = 1;
            current.y += 1;
            if current.y > width as i32 - 2 {
              next =
                Some(
                  FaceEdges::Parallel2 {
                    chunk     : chunk,
                    direction : direction,
                    lg_size   : lg_size,
                    width     : width,
                    current   : cgmath::Vector3::new(0, 1, 0),
                  }
                );
            }
          }
        },
        &mut FaceEdges::Parallel2 { chunk, direction, lg_size, width, ref mut current } => {
          let (x, (y, z)) = (direction, direction.perpendicular());
          let (xv, yv, zv) = (x.to_vec(), y.to_vec(), z.to_vec());
          let base = chunk.as_point * width as i32;
          r =
            edge::T {
              low_corner : base + xv*current.x + yv*current.y + zv*current.z,
              direction  : z,
              lg_size    : lg_size,
            };
          current.z += 1;
          if current.z > width as i32 - 2 {
            current.z = 0;
            current.y += 1;
            if current.y > width as i32 - 1 {
              next = Some(FaceEdges::Done);
            }
          }
        },
      }
      if let Some(next) = next {
        *self = next;
      }
      Some(r)
    }
  }

  pub enum EdgeEdges {
    Done,
    Active {
      chunk     : T,
      direction : edge::Direction,
      lg_size   : i16,
      width     : u32,
      current   : cgmath::Vector3<i32>,
    },
  }

  impl EdgeEdges {
    pub fn new(direction: edge::Direction, chunk: T, lg_size: i16) -> Self {
      let width: u32 = 1 << (chunk::LG_WIDTH as i16 - lg_size);
      if width <= 1 {
        EdgeEdges::Done
      } else {
        EdgeEdges::Active {
          chunk     : chunk,
          direction : direction,
          lg_size   : lg_size,
          width     : width,
          current   : cgmath::Vector3::new(0, 0, 0),
        }
      }
    }
  }

  impl Iterator for EdgeEdges {
    type Item = edge::T;
    fn next(&mut self) -> Option<Self::Item> {
      let mut next = None;
      let r;
      match self {
        &mut EdgeEdges::Done => return None,
        &mut EdgeEdges::Active { chunk, direction, lg_size, width, ref mut current } => {
          let (x, (y, z)) = (direction, direction.perpendicular());
          let (xv, yv, zv) = (x.to_vec(), y.to_vec(), z.to_vec());
          let base = chunk.as_point * width as i32;
          r =
            edge::T {
              low_corner : base + xv*current.x + yv*current.y + zv*current.z,
              direction  : direction,
              lg_size    : lg_size,
            };
          current.x += 1;
          if current.x > (width - 2) as i32 {
            next = Some(EdgeEdges::Done);
          }
        },
      }
      if let Some(next) = next {
        *self = next;
      }
      Some(r)
    }
  }
  pub fn inner_edges(position: T, lg_size: i16) -> InnerEdges {
    InnerEdges::new(position, lg_size)
  }

  pub fn face_edges(d: edge::Direction, position: T, lg_size: i16) -> FaceEdges {
    FaceEdges::new(d, position, lg_size)
  }

  pub fn edge_edges(d: edge::Direction, position: T, lg_size: i16) -> EdgeEdges {
    EdgeEdges::new(d, position, lg_size)
  }
}
