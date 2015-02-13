//! Data/code for applying updates to the client from other systems.

use client::Client;
use common::color::Color3;
use common::communicate::{ClientToServer, ServerToClient};
use light::Light;
use nalgebra::{Vec2, Vec3};
use std::cmp::partial_max;
use std::collections::hash_map::Entry::{Vacant, Occupied};
use std::f32::consts::PI;
use std::num::Float;
use std::sync::mpsc::Sender;
use view_update::ClientToView;

#[derive(Clone)]
/// Updates the `View` can send the `Client`.
pub enum ViewToClient {
  /// Add to the player's walking acceleration.
  Walk(Vec3<f32>),
  /// Turn the client laterally and vertically.
  RotatePlayer(Vec2<f32>),
  /// Start the player jumping.
  StartJump,
  /// Stop the player jumping.
  StopJump,
  /// Halt the client.
  Quit,
}

/// Apply a `ViewToClient` update to a `Client`.
pub fn apply_view_to_client(up: ViewToClient, ups_to_server: &Sender<ClientToServer>) -> bool {
  match up {
    ViewToClient::Quit => {
      ups_to_server.send(ClientToServer::Quit).unwrap();
      return false;
    },
    ViewToClient::Walk(v) => {
      ups_to_server.send(ClientToServer::Walk(v)).unwrap();
    },
    ViewToClient::RotatePlayer(v) => {
      ups_to_server.send(ClientToServer::RotatePlayer(v)).unwrap();
    },
    ViewToClient::StartJump => {
      ups_to_server.send(ClientToServer::StartJump).unwrap();
    },
    ViewToClient::StopJump => {
      ups_to_server.send(ClientToServer::StopJump).unwrap();
    },
  }

  true
}

/// Apply a `ServerToClient` update to a `Client`.
pub fn apply_server_to_client(
  up: ServerToClient,
  client: &mut Client,
  ups_to_view: &Sender<ClientToView>,
) {
  match up {
    ServerToClient::UpdatePlayer(position) => {
      client.player_position = position;
      ups_to_view.send(ClientToView::MoveCamera(position)).unwrap();
    },
    ServerToClient::AddMob(id, v) => {
      ups_to_view.send(ClientToView::AddMob(id, v)).unwrap();
    },
    ServerToClient::UpdateMob(id, v) => {
      ups_to_view.send(ClientToView::UpdateMob(id, v)).unwrap();
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
      let rel_position = Vec3::new(c, s, 0.0) * radius;

      ups_to_view.send(ClientToView::SetPointLight(
        Light {
          position: client.player_position + rel_position,
          intensity: sun_color,
        }
      )).unwrap();

      let ambient_light = partial_max(0.4, s / 2.0).unwrap();

      ups_to_view.send(ClientToView::SetAmbientLight(
        Color3::of_rgb(
          sun_color.r * ambient_light,
          sun_color.g * ambient_light,
          sun_color.b * ambient_light,
        ),
      )).unwrap();

      ups_to_view.send(ClientToView::SetClearColor(sun_color)).unwrap();
    },
    // TODO: Is there a race where this block is stale by the time it gets to the client?
    ServerToClient::AddBlock(position, block, lod) => {
      match client.loaded_blocks.entry(position) {
        Vacant(entry) => {
          entry.insert((block.clone(), lod));
        },
        Occupied(mut entry) => {
          {
            let &(ref prev_block, prev_lod) = entry.get();
            for &id in prev_block.ids.iter() {
              ups_to_view.send(ClientToView::RemoveTerrain(id)).unwrap();
            }
            ups_to_view.send(
              ClientToView::RemoveBlockData(position, prev_lod)
            ).unwrap();
          }
          entry.insert((block.clone(), lod));
        },
      };

      if !block.ids.is_empty() {
        ups_to_view.send(ClientToView::AddBlock(position, block, lod)).unwrap();
      }
    },
  }
}
