use std::sync::mpsc::channel;
use std::thread;
use time;

use common::communicate::{ClientToServer, ServerToClient};
use common::socket::SendSocket;

use server::Server;
use update_gaia::{ServerToGaia, LoadReason};

#[inline]
pub fn apply_client_update<UpdateGaia>(
  server: &Server,
  update_gaia: &mut UpdateGaia,
  update: ClientToServer,
) where
  UpdateGaia: FnMut(ServerToGaia),
{
  match update {
    ClientToServer::Init(client_url) => {
      info!("Sending to {}.", client_url);

      let (to_client_send, to_client_recv) = channel();
      let client_thread = {
        thread::scoped(move || {
          let mut socket = SendSocket::new(client_url.as_slice());
          while let Some(msg) = to_client_recv.recv().unwrap() {
            let now = time::precise_time_ns();
            socket.write(msg);
            println!("send took {}", time::precise_time_ns() - now);
          }
        })
      };
      let player_position = server.player.lock().unwrap().position;

      to_client_send.send(
        Some(ServerToClient::UpdatePlayer(player_position))
      ).unwrap();
      server.inform_client(
        &mut |msg| to_client_send.send(Some(msg)).unwrap()
      );

      *server.to_client.lock().unwrap() = Some((to_client_send, client_thread));
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
