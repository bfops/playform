use std;
use std::io::Write;
use std::sync::{Mutex};
use stopwatch;
use thread_scoped;

use common::protocol;

use audio_loader;
use audio_thread;
use client;
use server;
use record_book;
use update_thread::update_thread;
use view::thread::view_thread;

#[allow(missing_docs)]
pub fn run(listen_url: &str, server_url: &str) {
  let view_updates0 = Mutex::new(std::collections::VecDeque::new());
  let view_updates1 = Mutex::new(std::collections::VecDeque::new());
  let audio_updates = Mutex::new(std::collections::VecDeque::new());

  let quit = Mutex::new(false);
  let quit = &quit;

  let server = server::new(&server_url, &listen_url);

  let client = connect_client(&listen_url, &server);
  let client = &client;

  {
    let monitor_thread = {
      unsafe {
        thread_scoped::scoped(|| {
          while !*quit.lock().unwrap() {
            info!("Outstanding voxel updates: {}", client.terrain.lock().unwrap().queued_update_count());
            info!("Outstanding view0 updates: {}", view_updates0.lock().unwrap().len());
            info!("Outstanding view1 updates: {}", view_updates1.lock().unwrap().len());
            std::thread::sleep(std::time::Duration::from_secs(1));
          }
        })
      }
    };

    let audio_thread = {
      let audio_updates = &audio_updates;
      unsafe {
        thread_scoped::scoped(move || {
          audio_thread::audio_thread(
            quit,
            &mut || { audio_updates.lock().unwrap().pop_front() },
          );
        })
      }
    };

    audio_updates.lock().unwrap().push_back(audio_thread::Message::PlayLoop(audio_loader::SoundId::Rainforest));

    let update_thread = {
      let client = &client;
      let view_updates0 = &view_updates0;
      let view_updates1 = &view_updates1;
      let audio_updates = &audio_updates;
      let server = server.clone();
      unsafe {
        thread_scoped::scoped(move || {
          update_thread(
            quit,
            client,
            &mut || { server.listen.try() },
            &mut |up| { view_updates0.lock().unwrap().push_back(up) },
            &mut |up| { view_updates1.lock().unwrap().push_back(up) },
            &mut |up| { audio_updates.lock().unwrap().push_back(up) },
  	        &mut |up| { server.talk.tell(&up) },
            &mut |msg| {
              *client.pending_terrain_requests.lock().unwrap() -= 1;
              client.terrain.lock().unwrap().enqueue(msg);
            },
          );

          let mut recorded = record_book::thread_local::clone();
          recorded.chunk_loads.sort_by(|x, y| x.loaded_at.cmp(&y.loaded_at));

          let mut file = std::fs::File::create("chunk_loads.out").unwrap();

          file.write_all(b"records = [").unwrap();
          for (i, record) in recorded.chunk_loads.iter().enumerate() {
            if i > 0 {
              file.write_all(b", ").unwrap();
            }
            file.write_fmt(format_args!("[{}; {}; {}; {}]", record.requested_at, record.responded_at, record.processed_at, record.loaded_at)).unwrap();
          }
          file.write_all(b"];\n").unwrap();
          file.write_fmt(format_args!("plot([1:{}], records);", recorded.chunk_loads.len())).unwrap();

          stopwatch::clone()
        })
      }
    };

    {
      let client = &client;
      let server = server.clone();
      view_thread(
        client,
        &mut || { view_updates0.lock().unwrap().pop_front() },
        &mut || { view_updates1.lock().unwrap().pop_front() },
        &mut |server_update| { server.talk.tell(&server_update) },
      );

      stopwatch::clone().print();
    }

    // View thread returned, so we got a quit event.
    *quit.lock().unwrap() = true;

    audio_thread.join();
    monitor_thread.join();

    let stopwatch = update_thread.join();

    stopwatch.print();
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
