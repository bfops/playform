use common::communicate::{ClientToServer, ServerToClient, TerrainBlockSend, spark_socket_sender};
use common::process_events::process_channel;
use common::stopwatch::TimerSet;
use gaia_thread::{ServerToGaia, LoadReason};
use nanomsg::Endpoint;
use server::Server;
use std::old_io::timer;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::Mutex;
use std::time::duration::Duration;

pub fn client_thread(
  client_endpoints: &mut Vec<Endpoint>,
  server: &Server,
  ups_from_client: &Receiver<ClientToServer>,
  ups_to_gaia: &Mutex<Sender<ServerToGaia>>,
) {
  let timers = TimerSet::new();
  let timers = &timers;

  // TODO: Proper exit semantics for this thread.
  loop {
    process_channel(
      ups_from_client,
      |update| {
        match update {
          ClientToServer::Init(client_url) => {
            let (client, endpoint) = spark_socket_sender(client_url);
            client_endpoints.push(endpoint);
            let player_position = server.player.lock().unwrap().position;
            server.to_client.lock().unwrap().as_mut().map(|client| {
              client.send(ServerToClient::UpdatePlayer(player_position)).unwrap();
            });
            server.inform_client(&client);
            *server.to_client.lock().unwrap() = Some(client);
          },
          ClientToServer::StartJump => {
            let mut player = server.player.lock().unwrap();
            if !player.is_jumping {
              player.is_jumping = true;
              // this 0.3 is duplicated in a few places
              player.accel.y = player.accel.y + 0.3;
            }
          },
          ClientToServer::StopJump => {
            let mut player = server.player.lock().unwrap();
            if player.is_jumping {
              player.is_jumping = false;
              // this 0.3 is duplicated in a few places
              player.accel.y = player.accel.y - 0.3;
            }
          },
          ClientToServer::Walk(v) => {
            let mut player = server.player.lock().unwrap();
            player.walk(v);
          },
          ClientToServer::RotatePlayer(v) => {
            let mut player = server.player.lock().unwrap();
            player.rotate_lateral(v.x);
            player.rotate_vertical(v.y);
          },
          ClientToServer::RequestBlock(position, lod) => {
            timers.time("update.request_block", || {
              let terrain_game_loader = server.terrain_game_loader.lock().unwrap();
              let block = terrain_game_loader.terrain.all_blocks.get(&position);
              match block {
                None => {
                  ups_to_gaia.lock().unwrap().send(
                    ServerToGaia::Load(position, lod, LoadReason::ForClient)
                  ).unwrap();
                },
                Some(block) => {
                  match block.lods.get(lod.0 as usize) {
                    Some(&Some(ref block)) => {
                      server.to_client.lock().unwrap().as_mut().map(|client| {
                        client.send(
                          ServerToClient::AddBlock(TerrainBlockSend {
                            position: position,
                            block: block.clone(),
                            lod: lod,
                          })
                        ).unwrap();
                      });
                    },
                    _ => {
                      ups_to_gaia.lock().unwrap().send(
                        ServerToGaia::Load(position, lod, LoadReason::ForClient)
                      ).unwrap();
                    },
                  }
                },
              }
            })
          },
        };

        true
      },
    );

    timer::sleep(Duration::milliseconds(1));
  }
}
