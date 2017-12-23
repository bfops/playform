//! entry point

use std;
use std::io::Write;
use std::sync::{Mutex};
use stopwatch;
use thread_scoped;

use common::protocol;

use client;
use record_book;
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
  unsafe {
    let _update_thread =
      thread_scoped::scoped(move || {
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

        let mut recorded = record_book::thread_local::clone();
        recorded.chunk_loads.sort_by(|x, y| x.loaded_time_ns.cmp(&y.loaded_time_ns));

        let mut file = std::fs::File::create("chunk_loads.out").unwrap();

        file.write_all(b"records = [").unwrap();
        for (i, record) in recorded.chunk_loads.iter().enumerate() {
          if i > 0 {
            file.write_all(b", ").unwrap();
          }
          let record_book::ChunkLoad { time_requested_ns, response_time_ns, stored_time_ns, loaded_time_ns } = *record;
          file.write_fmt(format_args!("[{}; {}; {}; {}]", time_requested_ns, response_time_ns, stored_time_ns, loaded_time_ns)).unwrap();
        }
        file.write_all(b"];\n").unwrap();
        file.write_fmt(format_args!("plot([1:{}], records);", recorded.chunk_loads.len())).unwrap();

        stopwatch::clone()
      });
  }
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
