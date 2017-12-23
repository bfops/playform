//! entry point

use std::sync::{Mutex};

use common::protocol;

use client;
use server;
use terrain;
use update_thread::update_thread;

#[allow(missing_docs)]
pub fn run(listen_url: &str, server_url: &str) {
  let quit = Mutex::new(false);
  let quit = &quit;

  let server = server::new(&server_url, &listen_url);

  let client = connect_client(&listen_url, &server);
  let client = &client;

  let client = &client;
  let server = server.clone();
  update_thread(
    quit,
    client,
    &mut || { server.listen.try() },
    &mut |_| { },
    &mut |_| { },
    &mut |_| { },
    &mut |up| { server.talk.tell(&up) },
    &mut |msg| {
      match msg {
        terrain::Load::Voxels { time_requested: None, .. } => {},
        terrain::Load::Voxels { time_requested: Some(_), .. } => {
          *client.pending_terrain_requests.lock().unwrap() -= 1;
        }
      };
      client.terrain.lock().unwrap().enqueue(msg);
    },
  );
}

fn connect_client(listen_url: &str, server: &server::T) -> client::T {
  // TODO: Consider using RPCs to solidify the request-response patterns.
  server.talk.tell(&protocol::ClientToServer::Init(listen_url.to_owned()));
  loop {
    match server.listen.wait() {
      protocol::ServerToClient::LeaseId(client_id) => {
        server.talk.tell(&protocol::ClientToServer::AddPlayer(client_id));
        let client_id = client_id;
        loop {
          match server.listen.wait() {
            protocol::ServerToClient::PlayerAdded(player_id, position) => {
              return client::new(client_id, player_id, position);
            },
            msg => {
              // Ignore other messages in the meantime.
              warn!("Ignoring: {:?}", msg);
            },
          }
        }
      },
      msg => {
        // Ignore other messages in the meantime.
        warn!("Ignoring: {:?}", msg);
      },
    }
  }
}
