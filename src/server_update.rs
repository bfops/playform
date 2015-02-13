use client_update::ServerToClient;
use lod::{LOD, LODIndex};
use gaia_update::{ServerToGaia, LoadReason};
use nalgebra::{Vec2, Vec3};
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
    server: &mut Server,
    ups_to_client: &Sender<ServerToClient>,
    ups_to_gaia: &Sender<ServerToGaia>,
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
        let terrain = server.terrain_game_loader.terrain.lock().unwrap();
        let block = terrain.all_blocks.get(&position);
        match block {
          None => {
            ups_to_gaia.send(
              ServerToGaia::Load(position, lod, LoadReason::ForClient)
            ).unwrap();
          },
          Some(block) => {
            match block.lods.get(lod.0 as usize) {
              Some(&Some(ref block)) => {
                ups_to_client.send(
                  ServerToClient::AddBlock(position, block.clone(), lod)
                ).unwrap();
              },
              _ => {
                ups_to_gaia.send(
                  ServerToGaia::Load(position, lod, LoadReason::ForClient)
                ).unwrap();
              },
            }
          },
        }
      },
    }

    true
  }
}

unsafe impl Send for ClientToServer {}

pub enum GaiaToServer {
  Loaded(BlockPosition, LODIndex, LoadReason),
}

impl GaiaToServer {
  pub fn apply(
    self,
    timers: &TimerSet,
    server: &mut Server,
    ups_to_client: &Sender<ServerToClient>,
    ups_to_gaia: &Sender<ServerToGaia>,
  ) {
    // TODO: Maybe have a common "fetch and do X with block-that-I-assert-exists".

    match self {
      GaiaToServer::Loaded(position, lod_index, load_reason) => {
        match load_reason {
          LoadReason::Local(owner) => {
            server.terrain_game_loader.load(
              timers,
              &mut server.id_allocator,
              &mut server.physics,
              &position,
              LOD::LodIndex(lod_index),
              owner,
              ups_to_gaia,
            );
          },
          LoadReason::ForClient => {
            let terrain = server.terrain_game_loader.terrain.lock().unwrap();
            let block = terrain.all_blocks.get(&position).unwrap();
            let block = block.lods[lod_index.0 as usize].as_ref().unwrap();
            ups_to_client.send(
              ServerToClient::AddBlock(position, block.clone(), lod_index)
            ).unwrap();
          },
        }
      },
    };
  }
}
