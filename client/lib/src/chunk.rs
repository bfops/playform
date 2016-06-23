use cgmath;

use common::voxel;

use edge;

pub use common::chunk::*;

pub fn containing(voxel: &voxel::bounds::T) -> Position {
  let lg_ratio = LG_WIDTH - voxel.lg_size;
  Position::new(
    voxel.x >> lg_ratio,
    voxel.y >> lg_ratio,
    voxel.z >> lg_ratio,
  )
}

pub struct Edges<'a> {
  chunk     : &'a Position,
  current   : cgmath::Vector3<i32>,
  direction : edge::Direction,
  done      : bool,
}

impl<'a> Edges<'a> {
  #[allow(missing_docs)]
  pub fn new<'b: 'a>(chunk: &'b Position) -> Self {
    Edges {
      chunk     : chunk,
      current   : cgmath::Vector3::new(-1, 0, 0),
      direction : edge::Direction::X,
      done      : false,
    }
  }
}

impl<'a> Iterator for Edges<'a> {
  type Item = edge::T;
  fn next(&mut self) -> Option<Self::Item> {
    if self.done {
      return None
    }

    macro_rules! load(
      ($x:ident, $y:ident, $z:ident) => {{
        use cgmath::Point;
        let r =
          Some(edge::T {
            low_corner : self.chunk.coords.add_v(&self.current).mul_s(WIDTH as i32),
            lg_size    : self.chunk.lg_voxel_size,
            direction  : self.direction,
          });

        self.current.$z += 1;
        if (self.current.$z as u32) <= WIDTH { return r }
        self.current.$z = 0;

        self.current.$y += 1;
        if (self.current.$y as u32) <= WIDTH { return r }
        self.current.$y = 0;

        self.current.$x += 1;
        if (self.current.$x as u32) <= WIDTH { return r }

        r
      }}
    );

    match self.direction {
      edge::Direction::X => {
        let r = load!(x, y, z);
        self.direction = edge::Direction::Y;
        self.current = cgmath::Vector3::new(0, -1, 0);
        r
      },
      edge::Direction::Y => {
        let r = load!(y, x, z);
        self.direction = edge::Direction::Z;
        self.current = cgmath::Vector3::new(0, 0, -1);
        r
      },
      edge::Direction::Z => {
        let r = load!(y, x, z);
        self.done = true;
        r
      },
    }
  }
}

pub fn edges<'a>(position: &'a Position) -> Edges<'a> {
  Edges::new(position)
}
