pub use common::chunk::{T, LG_WIDTH, WIDTH};

pub mod position {
  use cgmath;

  use chunk;
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
    assert!(lg_ratio > 0);
    T {
      as_point :
        cgmath::Point3::new(
          voxel.x >> lg_ratio,
          voxel.y >> lg_ratio,
          voxel.z >> lg_ratio,
        ),
    }
  }
}
