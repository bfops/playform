use nalgebra::{Vec2, Vec3};
use world::World;

#[derive(Debug, Clone)]
pub enum WorldUpdate {
  Walk(Vec3<f32>),
  RotatePlayer(Vec2<f32>),
  StartJump,
  StopJump,
  Quit,
}

impl WorldUpdate {
  pub fn apply(self, world: &mut World) -> bool {
    match self {
      WorldUpdate::Quit => {
        return false;
      },
      WorldUpdate::StartJump => {
        if !world.player.is_jumping {
          world.player.is_jumping = true;
          // this 0.3 is duplicated in a few places
          world.player.accel.y = world.player.accel.y + 0.3;
        }
      },
      WorldUpdate::StopJump => {
        if world.player.is_jumping {
          world.player.is_jumping = false;
          // this 0.3 is duplicated in a few places
          world.player.accel.y = world.player.accel.y - 0.3;
        }
      },
      WorldUpdate::Walk(v) => {
        world.player.walk(v);
      },
      WorldUpdate::RotatePlayer(v) => {
        world.player.rotate_lateral(v.x);
        world.player.rotate_vertical(v.y);
      },
    }

    return true;
  }
}

unsafe impl Send for WorldUpdate {}
