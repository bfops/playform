use cgmath::{Point, Vector, Vector3};
use std::cmp::partial_max;
use std::f32::consts::PI;
use std::num::Float;

use common::block_position::BlockPosition;
use common::color::Color3;
use common::communicate::{ClientToServer, ServerToClient, TerrainBlockSend};
use common::surroundings_loader::LODChange;

use client::Client;
use light::Light;
use load_terrain::lod_index;
use view_update::ClientToView;

pub fn apply_server_update<UpdateView, UpdateServer, QueueBlock>(
  client: &Client,
  update_view: &mut UpdateView,
  update_server: &mut UpdateServer,
  queue_block: &mut QueueBlock,
  update: ServerToClient,
) where
  UpdateView: FnMut(ClientToView),
  UpdateServer: FnMut(ClientToServer),
  QueueBlock: FnMut(TerrainBlockSend),
{
  match update {
    ServerToClient::LeaseId(_) => {
      warn!("Client ID has already been leased.");
    },
    ServerToClient::PlayerAdded(id, _) => {
      warn!("Unexpected PlayerAdded event: {:?}.", id);
    },
    ServerToClient::UpdatePlayer(player_id, position) => {
      if player_id != client.player_id {
        return
      }

      *client.player_position.lock().unwrap() = position;
      update_view(ClientToView::MoveCamera(position));

      let position = BlockPosition::from_world_position(&position);
      client.surroundings_loader.lock().unwrap().update(
        position,
        |lod_change| {
          match lod_change {
            LODChange::Load(block_position, distance) => {
              let lod = lod_index(distance);
              update_server(ClientToServer::RequestBlock(block_position, lod));
            },
            LODChange::Unload(block_position) => {
              // The block removal code is duplicated elsewhere.
    
              client.loaded_blocks
                .lock().unwrap()
                .remove(&block_position)
                // If it wasn't loaded, don't unload anything.
                .map(|(block, prev_lod)| {
                  for id in block.ids.iter() {
                    update_view(ClientToView::RemoveTerrain(*id));
                  }
    
                  update_view(ClientToView::RemoveBlockData(block_position, prev_lod));
                });
            },
          };
        },
      );
    },
    ServerToClient::AddMob(id, v) => {
      update_view(ClientToView::AddMob(id, v));
    },
    ServerToClient::UpdateMob(id, v) => {
      update_view(ClientToView::UpdateMob(id, v));
    },
    ServerToClient::UpdateSun(fraction) => {
      // Convert to radians.
      let angle = fraction * 2.0 * PI;
      let (s, c) = angle.sin_cos();

      let sun_color =
        Color3::of_rgb(
          c.abs(),
          (s + 1.0) / 2.0,
          (s * 0.75 + 0.25).abs(),
        );

      let radius = 1024.0;
      let rel_position = Vector3::new(c, s, 0.0);
      rel_position.mul_s(radius);

      update_view(ClientToView::SetPointLight(
        Light {
          position: client.player_position.lock().unwrap().add_v(&rel_position),
          intensity: sun_color,
        }
      ));

      let ambient_light = partial_max(0.4, s / 2.0).unwrap();

      update_view(ClientToView::SetAmbientLight(
        Color3::of_rgb(
          sun_color.r * ambient_light,
          sun_color.g * ambient_light,
          sun_color.b * ambient_light,
        ),
      ));

      update_view(ClientToView::SetClearColor(sun_color));
    },
    ServerToClient::AddBlock(block) => {
      queue_block(block);
    },
  }
}
