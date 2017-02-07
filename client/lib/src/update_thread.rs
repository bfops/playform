//! The main thread that processes updates from the client and the server and dispatches updates to other systems.

use std::sync::Mutex;
use stopwatch;
use time;

use common::protocol;
use common::surroundings_loader;
use common::surroundings_loader::LoadType;

use audio_thread;
use chunk;
use chunk_stats;
use client;
use lod;
use server_update::apply_server_update;
use terrain;
use view;

const MAX_OUTSTANDING_TERRAIN_REQUESTS: u32 = 1 << 7;

#[allow(missing_docs)]
pub fn update_thread<RecvServer, UpdateView0, UpdateView1, UpdateAudio, UpdateServer, EnqueueTerrainLoad>(
  quit                 : &Mutex<bool>,
  client               : &client::T,
  recv_server          : &mut RecvServer,
  update_view0         : &mut UpdateView0,
  update_view1         : &mut UpdateView1,
  update_audio         : &mut UpdateAudio,
  update_server        : &mut UpdateServer,
  enqueue_terrain_load : &mut EnqueueTerrainLoad,
) where
  RecvServer         : FnMut() -> Option<protocol::ServerToClient>,
  UpdateView0        : FnMut(view::update::T),
  UpdateView1        : FnMut(view::update::T),
  UpdateAudio        : FnMut(audio_thread::Message),
  UpdateServer       : FnMut(protocol::ClientToServer),
  EnqueueTerrainLoad : FnMut(terrain::Load),
{
  let mut chunk_stats = chunk_stats::new();

  'update_loop: loop {
    let should_quit = *quit.lock().unwrap();
    if should_quit {
      break 'update_loop
    } else {
      stopwatch::time("update_iteration", || {
        stopwatch::time("process_server_updates", || {
          process_server_updates(client, recv_server, update_view0, update_audio, update_server, enqueue_terrain_load);
        });

        stopwatch::time("update_surroundings", || {
          update_surroundings(client, &mut chunk_stats, update_view1, update_server);
        });

        stopwatch::time("process_voxel_updates", || {
          process_voxel_updates(client, &mut chunk_stats, update_view1);
        });
      })
    }
  }

  debug!("Printing chunk stats");
  chunk_stats.output_to("vram_chunk_loads.out");
}

#[inline(never)]
fn update_surroundings<UpdateView, UpdateServer>(
  client        : &client::T,
  chunk_stats   : &mut chunk_stats::T,
  update_view   : &mut UpdateView,
  update_server : &mut UpdateServer,
) where
  UpdateView   : FnMut(view::update::T),
  UpdateServer : FnMut(protocol::ClientToServer),
{
  let start = time::precise_time_ns();
  let mut i = 0;
  let load_position = {
    let load_position = *client.load_position.lock().unwrap();
    load_position.unwrap_or_else(|| *client.player_position.lock().unwrap())
  };
  let load_position = chunk::position::of_world_position(&load_position);
  let mut surroundings_loader = client.surroundings_loader.lock().unwrap();
  let mut updates = surroundings_loader.updates(load_position.as_pnt()) ;
  loop {
    if *client.pending_terrain_requests.lock().unwrap() >= MAX_OUTSTANDING_TERRAIN_REQUESTS {
      trace!("update loop breaking");
      break;
    }

    let chunk_position;
    let load_type;
    match updates.next() {
      None => break,
      Some((b, l)) => {
        chunk_position = chunk::position::of_pnt(&b);
        load_type = l;
      },
    }

    debug!("chunk surroundings");
    let distance =
      surroundings_loader::distance_between(
        load_position.as_pnt(),
        chunk_position.as_pnt(),
      );
    match load_type {
      LoadType::Load => {
        stopwatch::time("update_thread.load_chunk", || {
          trace!("Loading distance {}", distance);
          let new_lod = lod::of_distance(distance as u32);
          let load_state = client.terrain.lock().unwrap().load_state(&chunk_position);
          if load_state == Some(new_lod) {
            debug!("Not re-loading {:?} at {:?}", chunk_position, new_lod);
          } else {
            load_or_request_chunk(client, chunk_stats, update_server, update_view, &chunk_position, new_lod);
          }
        })
      },
      LoadType::Downgrade => {
        stopwatch::time("update_thread.update_chunk", || {
          let new_lod = lod::of_distance(distance as u32);
          let load_state = client.terrain.lock().unwrap().load_state(&chunk_position);
          let is_downgrade = load_state.map(|lod| new_lod < lod) == Some(true);
          if is_downgrade {
            load_or_request_chunk(client, chunk_stats, update_server, update_view, &chunk_position, new_lod);
          } else {
            trace!("Not updating {:?} at {:?}", chunk_position, new_lod);
          }
        })
      },
      LoadType::Unload => {
        stopwatch::time("update_thread.unload", || {
          client.terrain.lock().unwrap().unload(update_view, &chunk_position);
        })
      },
    };

    if i >= 10 {
      i -= 10;
      if time::precise_time_ns() - start >= 1_000_000 {
        break
      }
    }
    i += 1;
  }
}

fn load_or_request_chunk<UpdateServer, UpdateView>(
  client         : &client::T,
  chunk_stats    : &mut chunk_stats::T,
  update_server  : &mut UpdateServer,
  update_view    : &mut UpdateView,
  chunk_position : &chunk::position::T,
  lod            : lod::T,
) where
  UpdateServer: FnMut(protocol::ClientToServer),
  UpdateView: FnMut(view::update::T),
{
  let mut terrain = client.terrain.lock().unwrap();
  let rng = &mut *client.rng.lock().unwrap();
  let r =
    terrain.load_chunk(
      &client.id_allocator,
      &mut *rng,
      chunk_stats,
      update_view,
      chunk_position,
      lod,
    );
  match r {
    Ok(()) => {},
    Err(voxels) => {
      update_server(
        protocol::ClientToServer::RequestVoxels {
          time_requested_ns : time::precise_time_ns(),
          client_id       : client.id,
          voxels          : voxels,
        }
      );
      *client.pending_terrain_requests.lock().unwrap() += 1;
    },
  }
}

#[inline(never)]
fn process_voxel_updates<UpdateView>(
  client      : &client::T,
  chunk_stats : &mut chunk_stats::T,
  update_view : &mut UpdateView,
) where
  UpdateView: FnMut(view::update::T),
{
  let terrain = &mut *client.terrain.lock().unwrap();
  let rng = &mut *client.rng.lock().unwrap();
  terrain.tick(
    &client.id_allocator,
    rng,
    chunk_stats,
    update_view,
    &*client.player_position.lock().unwrap(),
  );
}

#[inline(never)]
fn process_server_updates<RecvServer, UpdateView, UpdateAudio, UpdateServer, EnqueueTerrainLoad>(
  client               : &client::T,
  recv_server          : &mut RecvServer,
  update_view          : &mut UpdateView,
  update_audio         : &mut UpdateAudio,
  update_server        : &mut UpdateServer,
  enqueue_terrain_load : &mut EnqueueTerrainLoad,
) where
  RecvServer         : FnMut() -> Option<protocol::ServerToClient>,
  UpdateView         : FnMut(view::update::T),
  UpdateAudio        : FnMut(audio_thread::Message),
  UpdateServer       : FnMut(protocol::ClientToServer),
  EnqueueTerrainLoad : FnMut(terrain::Load),
{
  let start = time::precise_time_ns();
  let mut i = 0;
  while let Some(up) = recv_server() {
    apply_server_update(
      client,
      update_view,
      update_audio,
      update_server,
      enqueue_terrain_load,
      up,
    );

    if i > 10 {
      i -= 10;
      if time::precise_time_ns() - start >= 1_000_000 {
        break
      }
    }
    i += 1;
  }
}
