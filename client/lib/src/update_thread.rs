use cgmath::{Aabb3, Point3};
use std::sync::Mutex;
use stopwatch;
use time;

use common::protocol;
use common::surroundings_loader;
use common::surroundings_loader::LoadType;
use common::voxel;

use audio_thread;
use chunk_position;
use client;
use lod;
use load_terrain;
use load_terrain::lod_index;
use record_book;
use server_update::apply_server_update;
use terrain_mesh;
use view_update;

const MAX_OUTSTANDING_TERRAIN_REQUESTS: u32 = 1;

#[allow(missing_docs)]
pub fn update_thread<RecvServer, RecvVoxelUpdates, UpdateView0, UpdateView1, UpdateAudio, UpdateServer, EnqueueChunkUpdates>(
  quit: &Mutex<bool>,
  client: &client::T,
  recv_server: &mut RecvServer,
  recv_voxel_updates: &mut RecvVoxelUpdates,
  update_view0: &mut UpdateView0,
  update_view1: &mut UpdateView1,
  update_audio: &mut UpdateAudio,
  update_server: &mut UpdateServer,
  enqueue_chunk_updates: &mut EnqueueChunkUpdates,
) where
  RecvServer: FnMut() -> Option<protocol::ServerToClient>,
  RecvVoxelUpdates: FnMut() -> Option<(Vec<(voxel::bounds::T, voxel::T)>, protocol::VoxelReason)>,
  UpdateView0: FnMut(view_update::T),
  UpdateView1: FnMut(view_update::T),
  UpdateAudio: FnMut(audio_thread::Message),
  UpdateServer: FnMut(protocol::ClientToServer),
  EnqueueChunkUpdates: FnMut(Vec<(voxel::bounds::T, voxel::T)>, protocol::VoxelReason),
{
  'update_loop: loop {
    let should_quit = *quit.lock().unwrap();
    if should_quit {
      break 'update_loop
    } else {
      stopwatch::time("update_iteration", || {
        stopwatch::time("process_server_updates", || {
          process_server_updates(client, recv_server, update_view0, update_audio, update_server, enqueue_chunk_updates);
        });

        stopwatch::time("update_surroundings", || {
          update_surroundings(client, update_view1, update_server);
        });

        stopwatch::time("process_voxel_updates", || {
          process_voxel_updates(client, recv_voxel_updates, update_view1);
        });
      })
    }
  }
}

#[inline(never)]
fn update_surroundings<UpdateView, UpdateServer>(
  client: &client::T,
  update_view: &mut UpdateView,
  update_server: &mut UpdateServer,
) where
  UpdateView: FnMut(view_update::T),
  UpdateServer: FnMut(protocol::ClientToServer),
{
  let start = time::precise_time_ns();
  let mut i = 0;
  let load_position = {
    let load_position = *client.load_position.lock().unwrap();
    load_position.unwrap_or_else(|| *client.player_position.lock().unwrap())
  };
  let load_position = chunk_position::of_world_position(&load_position);
  let mut surroundings_loader = client.surroundings_loader.lock().unwrap();
  let mut updates = surroundings_loader.updates(load_position.as_pnt()) ;
  loop {
    if *client.outstanding_terrain_requests.lock().unwrap() >= MAX_OUTSTANDING_TERRAIN_REQUESTS {
      trace!("update loop breaking");
      break;
    }

    let chunk_position;
    let load_type;
    match updates.next() {
      None => break,
      Some((b, l)) => {
        chunk_position = chunk_position::of_pnt(&b);
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
          let new_lod = lod_index(distance);
          let lod_change =
            client.loaded_chunks
            .lock().unwrap()
            .get(&chunk_position)
            .map(|&(_, lod)| lod != new_lod);
          if lod_change == Some(false) {
            debug!("Not re-loading {:?} at {:?}", chunk_position, new_lod);
          } else {
            load_or_request_chunk(client, update_server, update_view, chunk_position, new_lod);
          }
        })
      },
      LoadType::Update => {
        stopwatch::time("update_thread.update_chunk", || {
          let new_lod = lod_index(distance);
          let lod_change =
            client.loaded_chunks
            .lock().unwrap()
            .get(&chunk_position)
            .map(|&(_, lod)| new_lod < lod);
          if lod_change == Some(true) {
            load_or_request_chunk(client, update_server, update_view, chunk_position, new_lod);
          } else {
            trace!("Not updating {:?} at {:?}", chunk_position, new_lod);
          }
        })
      },
      LoadType::Unload => {
        stopwatch::time("update_thread.unload", || {
          // The chunk removal code is duplicated in load_terrain.

          client.loaded_chunks
          .lock().unwrap()
            .remove(&chunk_position)
            // If it wasn't loaded, don't unload anything.
            .map(|(chunk, _)| {
              for id in &chunk.grass_ids {
                update_view(view_update::RemoveGrass(*id));
              }
              for id in &chunk.ids {
                update_view(view_update::RemoveTerrain(*id));
              }
            });
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
  client: &client::T,
  update_server: &mut UpdateServer,
  update_view: &mut UpdateView,
  chunk_position: chunk_position::T,
  lod: lod::T,
) where
  UpdateServer: FnMut(protocol::ClientToServer),
  UpdateView: FnMut(view_update::T),
{
  if load_terrain::all_voxels_loaded(&client.chunk_voxels_loaded.lock().unwrap(), chunk_position, lod) {
    load_terrain::load_chunk(
      client,
      update_view,
      &chunk_position,
      lod,
    );
  } else {
    let voxel_size = 1 << terrain_mesh::LG_SAMPLE_SIZE[lod.0 as usize];
    update_server(
      protocol::ClientToServer::RequestVoxels {
        requested_at : time::precise_time_ns(),
        client_id    : client.id,
        voxels       :
          terrain_mesh::voxels_in(
            &Aabb3::new(
              Point3::new(
                (chunk_position.as_pnt().x << terrain_mesh::LG_WIDTH) - voxel_size,
                (chunk_position.as_pnt().y << terrain_mesh::LG_WIDTH) - voxel_size,
                (chunk_position.as_pnt().z << terrain_mesh::LG_WIDTH) - voxel_size,
              ),
              Point3::new(
                ((chunk_position.as_pnt().x + 1) << terrain_mesh::LG_WIDTH) + voxel_size,
                ((chunk_position.as_pnt().y + 1) << terrain_mesh::LG_WIDTH) + voxel_size,
                ((chunk_position.as_pnt().z + 1) << terrain_mesh::LG_WIDTH) + voxel_size,
              ),
            ),
            terrain_mesh::LG_SAMPLE_SIZE[lod.0 as usize],
          ),
      }
    );
    *client.outstanding_terrain_requests.lock().unwrap() += 1;
  }
}

#[inline(never)]
fn process_voxel_updates<RecvVoxelUpdates, UpdateView>(
  client             : &client::T,
  recv_voxel_updates : &mut RecvVoxelUpdates,
  update_view        : &mut UpdateView,
) where
  RecvVoxelUpdates: FnMut() -> Option<(Vec<(voxel::bounds::T, voxel::T)>, protocol::VoxelReason)>,
  UpdateView: FnMut(view_update::T),
{
  let start = time::precise_time_ns();
  while let Some((voxel_updates, reason)) = recv_voxel_updates() {
    let mut update_chunks = chunk_position::with_lod::set::new();
    let response_time = time::precise_time_ns();
    for (bounds, voxel) in voxel_updates {
      trace!("Got voxel at {:?}", bounds);
      load_terrain::load_voxel(
        client,
        voxel,
        &bounds,
        |chunk, lod| { update_chunks.insert((chunk, lod)); },
      );
    }

    let processed_time = time::precise_time_ns();

    for (chunk, lod) in update_chunks.into_iter() {
      load_terrain::load_chunk(
        client,
        update_view,
        &chunk,
        lod
      )
    }

    let chunk_loaded = time::precise_time_ns();

    match reason {
      protocol::VoxelReason::Updated => {},
      protocol::VoxelReason::Requested { at } => {
        *client.outstanding_terrain_requests.lock().unwrap() -= 1;
        debug!("Outstanding terrain requests: {}", *client.outstanding_terrain_requests.lock().unwrap());

        record_book::thread_local::push_chunk_load(
          record_book::ChunkLoad {
            requested_at : at,
            responded_at : response_time,
            processed_at : processed_time,
            loaded_at    : chunk_loaded,
          }
        );
      },
    }

    if time::precise_time_ns() - start >= 1_000_000 {
      break
    }
  }
}

#[inline(never)]
fn process_server_updates<RecvServer, UpdateView, UpdateAudio, UpdateServer, EnqueueChunkUpdates>(
  client                : &client::T,
  recv_server           : &mut RecvServer,
  update_view           : &mut UpdateView,
  update_audio          : &mut UpdateAudio,
  update_server         : &mut UpdateServer,
  enqueue_chunk_updates : &mut EnqueueChunkUpdates,
) where
  RecvServer          : FnMut() -> Option<protocol::ServerToClient>,
  UpdateView          : FnMut(view_update::T),
  UpdateAudio         : FnMut(audio_thread::Message),
  UpdateServer        : FnMut(protocol::ClientToServer),
  EnqueueChunkUpdates : FnMut(Vec<(voxel::bounds::T, voxel::T)>, protocol::VoxelReason),
{
  let start = time::precise_time_ns();
  let mut i = 0;
  while let Some(up) = recv_server() {
    apply_server_update(
      client,
      update_view,
      update_audio,
      update_server,
      enqueue_chunk_updates,
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
