use common::communicate::{ClientToServer, ServerToClient};
use common::socket::SendSocket;
use gaia_thread::{ServerToGaia, LoadReason};
use server::Server;
use std::sync::mpsc::Receiver;

pub fn client_thread<UpdateGaia>(
  server: &Server,
  ups_from_client: &Receiver<ClientToServer>,
  update_gaia: &mut UpdateGaia,
) where UpdateGaia: FnMut(ServerToGaia)
{
  // TODO: Proper exit semantics for this and other threads.
  loop {
    let update = ups_from_client.recv().unwrap();
    match update {
      ClientToServer::Init(client_url) => {
        info!("Sending to {}.", client_url);

        let to_client = SendSocket::<'static, _>::spawn(client_url.as_slice());
        let player_position = server.player.lock().unwrap().position;

        to_client.send(ServerToClient::UpdatePlayer(player_position));
        server.inform_client(&mut |msg| to_client.send(msg));

        *server.to_client.lock().unwrap() = Some(to_client);
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
        update_gaia(ServerToGaia::Load(position, lod, LoadReason::ForClient));
      },
    };
  }
}
