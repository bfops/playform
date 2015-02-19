use client::Client;
use common::block_position::BlockPosition;
use common::color::Color3;
use common::communicate::ServerToClient;
use common::process_events::process_channel;
use common::surroundings_loader::radius_between;
use light::Light;
use nalgebra::Vec3;
use std::cmp::partial_max;
use std::collections::hash_map::Entry::{Vacant, Occupied};
use std::f32::consts::PI;
use std::num::Float;
use std::old_io::timer;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::Mutex;
use std::time::duration::Duration;
use surroundings_thread::lod_index;
use view_update::ClientToView;

#[allow(missing_docs)]
pub fn server_thread(
  client: &Client,
  ups_from_server: &Receiver<ServerToClient>,
  ups_to_view: &Mutex<Sender<ClientToView>>,
) {
  loop {
    process_channel(
      ups_from_server,
      |update| {
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
          // TODO: Is there a race where this block is stale by the time it gets to the client?
          ServerToClient::AddBlock(position, block, lod) => {
            let player_position =
              BlockPosition::from_world_position(&client.player_position.lock().unwrap().clone());
            let distance = radius_between(&player_position, &position);
            if distance <= client.max_load_distance && lod_index(distance) == lod {
              match client.loaded_blocks.lock().unwrap().entry(position) {
                Vacant(entry) => {
                  entry.insert((block.clone(), lod));
                },
                Occupied(mut entry) => {
                  {
                    // The block removal code is duplicated elsewhere.

                    let &(ref prev_block, prev_lod) = entry.get();
                    for &id in prev_block.ids.iter() {
                      ups_to_view.lock().unwrap().send(ClientToView::RemoveTerrain(id)).unwrap();
                    }
                    ups_to_view.lock().unwrap().send(
                      ClientToView::RemoveBlockData(position, prev_lod)
                    ).unwrap();
                  }
                  entry.insert((block.clone(), lod));
                },
              };

              if !block.ids.is_empty() {
                ups_to_view.lock().unwrap().send(ClientToView::AddBlock(position, block, lod)).unwrap();
              }
            }
          },
        };
        true
      },
    );

    // TODO: Is sleep(1) much better than sleep(0)?
    timer::sleep(Duration::milliseconds(1));
  }
}
