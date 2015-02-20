use client::Client;
use common::color::Color3;
use common::communicate::{ServerToClient, TerrainBlockSend};
use light::Light;
use nalgebra::Vec3;
use std::cmp::partial_max;
use std::f32::consts::PI;
use std::num::Float;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::Mutex;
use view_update::ClientToView;

#[allow(missing_docs)]
pub fn server_thread(
  client: &Client,
  ups_from_server: &Receiver<ServerToClient>,
  ups_to_view: &Mutex<Sender<ClientToView>>,
  terrain_updates: &Sender<TerrainBlockSend>,
) {
  loop {
    let update = ups_from_server.recv().unwrap();
    match update {
      ServerToClient::UpdatePlayer(position) => {
        *client.player_position.lock().unwrap() = position;
        ups_to_view.lock().unwrap().send(ClientToView::MoveCamera(position)).unwrap();
      },
      ServerToClient::AddMob(id, v) => {
        ups_to_view.lock().unwrap().send(ClientToView::AddMob(id, v)).unwrap();
      },
      ServerToClient::UpdateMob(id, v) => {
        ups_to_view.lock().unwrap().send(ClientToView::UpdateMob(id, v)).unwrap();
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

        ups_to_view.lock().unwrap().send(ClientToView::SetPointLight(
          Light {
            position: *client.player_position.lock().unwrap() + rel_position,
            intensity: sun_color,
          }
        )).unwrap();

        let ambient_light = partial_max(0.4, s / 2.0).unwrap();

        ups_to_view.lock().unwrap().send(ClientToView::SetAmbientLight(
          Color3::of_rgb(
            sun_color.r * ambient_light,
            sun_color.g * ambient_light,
            sun_color.b * ambient_light,
          ),
        )).unwrap();

        ups_to_view.lock().unwrap().send(ClientToView::SetClearColor(sun_color)).unwrap();
      },
      ServerToClient::AddBlock(block) => {
        terrain_updates.send(block).unwrap();
      },
    };
  }
}
