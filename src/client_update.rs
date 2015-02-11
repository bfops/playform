use client::Client;
use color::Color3;
use light::Light;
use nalgebra::{Vec2, Vec3, Pnt3};
use server_update::ClientToServer;
use std::cmp::partial_max;
use std::collections::hash_map::Entry::{Vacant, Occupied};
use std::f32::consts::PI;
use std::num::Float;
use std::sync::mpsc::Sender;
use terrain::terrain_block::{BlockPosition, TerrainBlock};
use vertex::ColoredVertex;
use view_update::ClientToView;
use server::EntityId;

#[derive(Clone)]
pub enum ViewToClient {
  Walk(Vec3<f32>),
  RotatePlayer(Vec2<f32>),
  StartJump,
  StopJump,
  Quit,
}

impl ViewToClient {
  pub fn apply(self, ups_to_server: &Sender<ClientToServer>) -> bool {
    match self {
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
}

pub enum ServerToClient {
  UpdatePlayer(Pnt3<f32>),

  AddMob((EntityId, Vec<ColoredVertex>)),
  UpdateMob((EntityId, Vec<ColoredVertex>)),

  // The sun as a [0, 1) portion of its cycle.
  UpdateSun(f32),

  AddBlock((BlockPosition, TerrainBlock, u32)),
}

impl ServerToClient {
  pub fn apply(self, client: &mut Client, ups_to_view: &Sender<ClientToView>) {
    match self {
      ServerToClient::UpdatePlayer(position) => {
        client.player_position = position;
        ups_to_view.send(ClientToView::MoveCamera(position)).unwrap();
      },
      ServerToClient::AddMob(v) => {
        ups_to_view.send(ClientToView::AddMob(v)).unwrap();
      },
      ServerToClient::UpdateMob(v) => {
        ups_to_view.send(ClientToView::UpdateMob(v)).unwrap();
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
      ServerToClient::AddBlock((position, block, lod)) => {
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
                ClientToView::RemoveBlockData((position, prev_lod))
              ).unwrap();
            }
            entry.insert((block.clone(), lod));
          },
        };

        ups_to_view.send(ClientToView::AddBlock((position, block, lod))).unwrap();
      },
    }
  }
}
