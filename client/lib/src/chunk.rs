use cgmath;
use std;

use common::voxel;

use terrain_mesh;

pub use common::chunk::{T, LG_WIDTH, WIDTH};

pub fn containing(voxel: &voxel::bounds::T) -> position::T {
  let lg_ratio = LG_WIDTH as i16 - voxel.lg_size;
  assert!(lg_ratio > 0);
  position::T {
    as_point :
      cgmath::Point3::new(
        voxel.x >> lg_ratio,
        voxel.y >> lg_ratio,
        voxel.z >> lg_ratio,
      ),
  }
}

pub mod position {
  use cgmath;

  use terrain_mesh;

  pub use common::chunk::position::*;

  #[allow(missing_docs)]
  pub fn of_world_position(world_position: &cgmath::Point3<f32>) -> T {
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

    T {
      as_point:
        cgmath::Point3::new(
          convert_coordinate(world_position.x),
          convert_coordinate(world_position.y),
          convert_coordinate(world_position.z),
        ),
    }
  }
}
