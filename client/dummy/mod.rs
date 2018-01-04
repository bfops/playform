//! Client binary

#![deny(missing_docs)]

extern crate cgmath;
#[macro_use]
extern crate log;
extern crate thread_scoped;
extern crate time;

extern crate common;
extern crate client_lib;

use std::sync::{Mutex};

use common::protocol;

use client_lib::client;
use client_lib::server;
use client_lib::update_thread::update_thread;

#[allow(missing_docs)]
pub fn run(listen_url: &str, server_url: &str) {
  let quit = Mutex::new(false);
  let quit = &quit;

  let server = server::new(&server_url, &listen_url);

  let client = connect_client(&listen_url, &server);
  *client.load_position.lock().unwrap() = Some(cgmath::Point3::new(0.0, 512.0, 0.0));
  let client = &client;

  let loaded_count = Mutex::new(0);
  let start = time::precise_time_ns();

  let monitor_thread = {
    unsafe {
      thread_scoped::scoped(|| {
        while !*quit.lock().unwrap() {
          let now = time::precise_time_ns();
          let loaded_count = *loaded_count.lock().unwrap();
          info!("Chunks received: {}", loaded_count);
          info!("Chunk receive rate: {} Hz", loaded_count as f32 / (now - start) as f32 * 1e9);
          std::thread::sleep(std::time::Duration::from_secs(1));
        }
      })
    }
  };

  let update_thread = {
    let client       = &client;
    let loaded_count = &loaded_count;
    let server       = server.clone();
    unsafe {
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
            if let client_lib::terrain::Load::Voxels { time_requested: Some(_), .. } = msg {
              *loaded_count.lock().unwrap() += 1;
              *client.pending_terrain_requests.lock().unwrap() -= 1;
            }
          },
        );
      })
    }
  };

  while (time::precise_time_ns() - start) < 80e9 as u64 {
    std::thread::sleep(std::time::Duration::from_secs(1));
  }

  println!("{} bytes sent", server.talk.bytes_sent.load(::std::sync::atomic::Ordering::SeqCst));

  // View thread returned, so we got a quit event.
  *quit.lock().unwrap() = true;

  monitor_thread.join();
  update_thread.join();
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
