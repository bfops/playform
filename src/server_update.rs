use client_update::ServerToClient;
use lod::LODIndex;
use nalgebra::{Vec2, Vec3};
use opencl_context::CL;
use server::Server;
use std::sync::mpsc::Sender;
use stopwatch::TimerSet;
use terrain::terrain_block::BlockPosition;

#[derive(Debug, Clone)]
pub enum ClientToServer {
  Walk(Vec3<f32>),
  RotatePlayer(Vec2<f32>),
  StartJump,
  StopJump,
  RequestBlock(BlockPosition, LODIndex),
  Quit,
}

impl ClientToServer {
  pub fn apply(
    self,
    timers: &TimerSet,
    cl: &CL,
    server: &mut Server,
    server_to_client: &Sender<ServerToClient>,
  ) -> bool {
    match self {
      ClientToServer::Quit => {
        return false;
      },
      ClientToServer::StartJump => {
        if !server.player.is_jumping {
          server.player.is_jumping = true;
          // this 0.3 is duplicated in a few places
          server.player.accel.y = server.player.accel.y + 0.3;
        }
      },
      ClientToServer::StopJump => {
        if server.player.is_jumping {
          server.player.is_jumping = false;
          // this 0.3 is duplicated in a few places
          server.player.accel.y = server.player.accel.y - 0.3;
        }
      },
      ClientToServer::Walk(v) => {
        server.player.walk(v);
      },
      ClientToServer::RotatePlayer(v) => {
        server.player.rotate_lateral(v.x);
        server.player.rotate_vertical(v.y);
      },
      ClientToServer::RequestBlock(position, lod) => {
        server.terrain_game_loader.terrain.load(
          timers,
          cl,
          &server.terrain_game_loader.texture_generators[lod.0 as usize],
          &mut server.id_allocator,
          &position,
          lod,
          |block| {
            server_to_client.send(
              ServerToClient::AddBlock(position, block.clone(), lod)
            ).unwrap();
          },
        );
      },
    }

    true
  }
}

unsafe impl Send for ClientToServer {}
