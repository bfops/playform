use common::communicate::{ClientToServer, ServerToClient, spark_socket_sender};
use gaia_thread::{ServerToGaia, LoadReason};
use nanomsg::Endpoint;
use server::Server;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::Mutex;

pub fn client_thread(
  client_endpoints: &mut Vec<Endpoint>,
  server: &Server,
  ups_from_client: &Receiver<ClientToServer>,
  ups_to_gaia: &Mutex<Sender<ServerToGaia>>,
) {
  // TODO: Proper exit semantics for this and other threads.
  loop {
    let update = ups_from_client.recv().unwrap();
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
        ups_to_gaia.lock().unwrap().send(
          ServerToGaia::Load(position, lod, LoadReason::ForClient)
        ).unwrap();
      },
    };
  }
}
