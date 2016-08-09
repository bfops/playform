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

  pub struct InnerEdges<'a> {
    chunk     : &'a T,
    lg_size   : i16,
    width     : u32,
    current   : cgmath::Vector3<i32>,
    direction : edge::Direction,
    done      : bool,
  }

  impl<'a> InnerEdges<'a> {
    #[allow(missing_docs)]
    pub fn new<'b: 'a>(chunk: &'b T, lg_size: i16) -> Self {
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

  impl<'a> Iterator for InnerEdges<'a> {
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

  pub fn inner_edges(position: &T, lg_size: i16) -> InnerEdges {
    InnerEdges::new(position, lg_size)
  }
}
