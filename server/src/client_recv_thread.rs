use cgmath::{Point, Point3, Vector3, Aabb3};
use std::f32::consts::PI;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;
use time;

use common::communicate::{ClientToServer, ServerToClient};
use common::serialize as binary;
use common::socket::SendSocket;

use player::Player;
use server::{Client, Server};
use terrain::terrain;
use update_gaia::{ServerToGaia, LoadReason};

fn center(bounds: &Aabb3<f32>) -> Point3<f32> {
  bounds.min.add_v(&bounds.max.to_vec()).mul_s(1.0 / 2.0)
}

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
          let mut socket = SendSocket::new(client_url.as_slice(), Some(Duration::seconds(30)));
          while let Some(msg) = to_client_recv.recv().unwrap() {
            let now = time::precise_time_ns();
            // TODO: Don't do this allocation on every packet!
            let msg = binary::encode(&msg).unwrap();
            socket.write(msg.as_slice());
            let delta = time::precise_time_ns() - now;
            trace!("Send took {}ns", delta);
          }
        })
      };

      let client_id = server.client_allocator.lock().unwrap().allocate();
      to_client_send.send(Some(ServerToClient::LeaseId(client_id))).unwrap();

      let client =
        Client {
          sender: to_client_send,
          thread: client_thread,
        };
      server.clients.lock().unwrap().insert(client_id, client);
    },
    ClientToServer::Ping(client_id) => {
      server.clients.lock().unwrap()
        .get(&client_id)
        .unwrap()
        .sender
        .send(Some(ServerToClient::Ping(())))
        .unwrap();
    },
    ClientToServer::AddPlayer(client_id) => {
      let mut player =
        Player::new(
          server.id_allocator.lock().unwrap().allocate(),
          &server.owner_allocator,
        );

      let min = Point3::new(0.0, terrain::AMPLITUDE as f32, 4.0);
      let max = min.add_v(&Vector3::new(1.0, 2.0, 1.0));
      let bounds = Aabb3::new(min, max);
      server.physics.lock().unwrap().insert_misc(player.entity_id, bounds.clone());

      player.position = center(&bounds);
      player.rotate_lateral(PI / 2.0);

      let id = player.entity_id;
      let pos = player.position;

      server.players.lock().unwrap().insert(id, player);

      let clients = server.clients.lock().unwrap();
      let client = clients.get(&client_id).unwrap();
      client.sender.send(
        Some(ServerToClient::PlayerAdded(id, pos))
      ).unwrap();
    },
    ClientToServer::StartJump(player_id) => {
      let mut players = server.players.lock().unwrap();
      let player = players.get_mut(&player_id).unwrap();
      if !player.is_jumping {
        player.is_jumping = true;
        // this 0.3 is duplicated in a few places
        player.accel.y = player.accel.y + 0.3;
      }
    },
    ClientToServer::StopJump(player_id) => {
      let mut players = server.players.lock().unwrap();
      let player = players.get_mut(&player_id).unwrap();
      if player.is_jumping {
        player.is_jumping = false;
        // this 0.3 is duplicated in a few places
        player.accel.y = player.accel.y - 0.3;
      }
    },
    ClientToServer::Walk(player_id, v) => {
      let mut players = server.players.lock().unwrap();
      let mut player = players.get_mut(&player_id).unwrap();
      player.walk(v);
    },
    ClientToServer::RotatePlayer(player_id, v) => {
      let mut players = server.players.lock().unwrap();
      let mut player = players.get_mut(&player_id).unwrap();
      player.rotate_lateral(v.x);
      player.rotate_vertical(v.y);
    },
    ClientToServer::RequestBlock(client_id, position, lod) => {
      update_gaia(ServerToGaia::Load(position, lod, LoadReason::ForClient(client_id)));
    },
  };
}
