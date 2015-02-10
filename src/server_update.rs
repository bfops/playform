use nalgebra::{Vec2, Vec3};
use server::Server;

#[derive(Debug, Clone)]
pub enum ClientToServer {
  Walk(Vec3<f32>),
  RotatePlayer(Vec2<f32>),
  StartJump,
  StopJump,
  Quit,
}

impl ClientToServer {
  pub fn apply(self, world: &mut Server) -> bool {
    match self {
      ClientToServer::Quit => {
        return false;
      },
      ClientToServer::StartJump => {
        if !world.player.is_jumping {
          world.player.is_jumping = true;
          // this 0.3 is duplicated in a few places
          world.player.accel.y = world.player.accel.y + 0.3;
        }
      },
      ClientToServer::StopJump => {
        if world.player.is_jumping {
          world.player.is_jumping = false;
          // this 0.3 is duplicated in a few places
          world.player.accel.y = world.player.accel.y - 0.3;
        }
      },
      ClientToServer::Walk(v) => {
        world.player.walk(v);
      },
      ClientToServer::RotatePlayer(v) => {
        world.player.rotate_lateral(v.x);
        world.player.rotate_vertical(v.y);
      },
    }

    true
  }
}

unsafe impl Send for ClientToServer {}
