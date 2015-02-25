use cgmath::{Point, Vector, Vector3};
use std::cmp::partial_max;
use std::f32::consts::PI;
use std::num::Float;

use common::block_position::BlockPosition;
use common::color::Color3;
use common::communicate::{ServerToClient, TerrainBlockSend};

use client::Client;
use light::Light;
use view_update::ClientToView;

pub fn apply_server_update<UpdateView, UpdateSurroundings, LoadTerrain>(
  client: &Client,
  update_view: &mut UpdateView,
  update_surroundings: &mut UpdateSurroundings,
  load_terrain: &mut LoadTerrain,
  update: ServerToClient,
) where
  UpdateView: FnMut(ClientToView),
  UpdateSurroundings: FnMut(BlockPosition),
  LoadTerrain: FnMut(TerrainBlockSend),
{
  match update {
    ServerToClient::UpdatePlayer(position) => {
      *client.player_position.lock().unwrap() = position;
      update_surroundings(BlockPosition::from_world_position(&position));
      update_view(ClientToView::MoveCamera(position));
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
      load_terrain(block);
    },
  }
}
