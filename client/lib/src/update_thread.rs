use cgmath::{Aabb3, Point3};
use std::sync::Mutex;
use stopwatch;
use time;

use common::protocol;
use common::surroundings_loader;
use common::surroundings_loader::LoadType;
use common::voxel;

use block_position;
use client;
use lod;
use load_terrain;
use load_terrain::lod_index;
use record_book;
use server_update::apply_server_update;
use terrain_mesh;
use view_update::ClientToView;

const MAX_OUTSTANDING_TERRAIN_REQUESTS: u32 = 1 << 1;

pub fn update_thread<RecvServer, RecvVoxelUpdates, UpdateView0, UpdateView1, UpdateServer, EnqueueBlockUpdates>(
  quit: &Mutex<bool>,
  client: &client::T,
  recv_server: &mut RecvServer,
  recv_voxel_updates: &mut RecvVoxelUpdates,
  update_view0: &mut UpdateView0,
  update_view1: &mut UpdateView1,
  update_server: &mut UpdateServer,
  enqueue_block_updates: &mut EnqueueBlockUpdates,
) where
  RecvServer: FnMut() -> Option<protocol::ServerToClient>,
  RecvVoxelUpdates: FnMut() -> Option<(Option<u64>, Vec<(voxel::bounds::T, voxel::T)>, protocol::VoxelReason)>,
  UpdateView0: FnMut(ClientToView),
  UpdateView1: FnMut(ClientToView),
  UpdateServer: FnMut(protocol::ClientToServer),
  EnqueueBlockUpdates: FnMut(Option<u64>, Vec<(voxel::bounds::T, voxel::T)>, protocol::VoxelReason),
{
  'update_loop: loop {
    let should_quit = *quit.lock().unwrap();
    if should_quit {
      break 'update_loop
    } else {
      stopwatch::time("update_iteration", || {
        stopwatch::time("process_server_updates", || {
          process_server_updates(client, recv_server, update_view0, update_server, enqueue_block_updates);
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
  UpdateView: FnMut(ClientToView),
  UpdateServer: FnMut(protocol::ClientToServer),
{
  let start = time::precise_time_ns();
  let mut i = 0;
  let load_position = {
    let load_position = *client.load_position.lock().unwrap();
    load_position.unwrap_or_else(|| *client.player_position.lock().unwrap())
  };
  let load_position = block_position::of_world_position(&load_position);
  let mut surroundings_loader = client.surroundings_loader.lock().unwrap();
  let mut updates = surroundings_loader.updates(load_position.as_pnt()) ;
  loop {
    if*client.outstanding_terrain_requests.lock().unwrap() >= MAX_OUTSTANDING_TERRAIN_REQUESTS {
      trace!("update loop breaking");
      break;
    }

    let block_position;
    let load_type;
    match updates.next() {
      None => break,
      Some((b, l)) => {
        block_position = block_position::of_pnt(&b);
        load_type = l;
      },
    }

    debug!("block surroundings");
    let distance =
      surroundings_loader::distance_between(
        load_position.as_pnt(),
        block_position.as_pnt(),
      );
    match load_type {
      LoadType::Load => {
        stopwatch::time("update_thread.load_block", || {
          let new_lod = lod_index(distance);
          let lod_change =
            client.loaded_blocks
            .lock().unwrap()
            .get(&block_position)
            .map(|&(_, lod)| lod != new_lod);
          if lod_change != Some(false) {
            load_or_request_chunk(client, update_server, update_view, block_position, new_lod);
          } else {
            debug!("Not re-loading {:?} at {:?}", block_position, new_lod);
          }
        })
      },
      LoadType::Update => {
        stopwatch::time("update_thread.update_block", || {
          let new_lod = lod_index(distance);
          let lod_change =
            client.loaded_blocks
            .lock().unwrap()
            .get(&block_position)
            .map(|&(_, lod)| new_lod < lod);
          if lod_change == Some(true) {
            load_or_request_chunk(client, update_server, update_view, block_position, new_lod);
          } else {
            trace!("Not updating {:?} at {:?}", block_position, new_lod);
          }
        })
      },
      LoadType::Unload => {
        stopwatch::time("update_thread.unload", || {
          // The block removal code is duplicated elsewhere.

          client.loaded_blocks
          .lock().unwrap()
            .remove(&block_position)
            // If it wasn't loaded, don't unload anything.
            .map(|(block, _)| {
              for id in &block.ids {
                update_view(ClientToView::RemoveTerrain(*id));
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
  block_position: block_position::T,
  lod: lod::T,
) where
  UpdateServer: FnMut(protocol::ClientToServer),
  UpdateView: FnMut(ClientToView),
{
  if load_terrain::all_voxels_loaded(&client.block_voxels_loaded.lock().unwrap(), block_position, lod) {
    load_terrain::load_block(
      client,
      update_view,
      &block_position,
      lod,
    );
  } else {
    let voxel_size = 1 << terrain_mesh::LG_SAMPLE_SIZE[lod.0 as usize];
    update_server(
      protocol::ClientToServer::RequestVoxels(
        time::precise_time_ns(),
        client.id,
        terrain_mesh::voxels_in(
          &Aabb3::new(
            Point3::new(
              (block_position.as_pnt().x << terrain_mesh::LG_WIDTH) - voxel_size,
              (block_position.as_pnt().y << terrain_mesh::LG_WIDTH) - voxel_size,
              (block_position.as_pnt().z << terrain_mesh::LG_WIDTH) - voxel_size,
            ),
            Point3::new(
              ((block_position.as_pnt().x + 1) << terrain_mesh::LG_WIDTH) + voxel_size,
              ((block_position.as_pnt().y + 1) << terrain_mesh::LG_WIDTH) + voxel_size,
              ((block_position.as_pnt().z + 1) << terrain_mesh::LG_WIDTH) + voxel_size,
            ),
          ),
          terrain_mesh::LG_SAMPLE_SIZE[lod.0 as usize],
        ),
      )
    );
    *client.outstanding_terrain_requests.lock().unwrap() += 1;
  }
}

#[inline(never)]
fn process_voxel_updates<RecvVoxelUpdates, UpdateView>(
  client: &client::T,
  recv_voxel_updates: &mut RecvVoxelUpdates,
  update_view: &mut UpdateView,
) where
  RecvVoxelUpdates: FnMut() -> Option<(Option<u64>, Vec<(voxel::bounds::T, voxel::T)>, protocol::VoxelReason)>,
  UpdateView: FnMut(ClientToView),
{
  let start = time::precise_time_ns();
  while let Some((request_time, voxel_updates, reason)) = recv_voxel_updates() {
    let mut update_blocks = block_position::with_lod::set::new();
    let response_time = time::precise_time_ns();
    for (bounds, voxel) in voxel_updates {
      trace!("Got voxel at {:?}", bounds);
      load_terrain::load_voxel(
        client,
        voxel,
        &bounds,
        |block, lod| { update_blocks.insert((block, lod)); },
      );
    }

    let processed_time = time::precise_time_ns();

    for (block, lod) in update_blocks.into_iter() {
      load_terrain::load_block(
        client,
        update_view,
        &block,
        lod
      )
    }

    let block_loaded = time::precise_time_ns();

    match request_time {
      None => {},
      Some(request_time) => {
        record_book::thread_local::push_block_load(
          record_book::BlockLoad {
            requested_at: request_time,
            responded_at: response_time,
            processed_at: processed_time,
            loaded_at: block_loaded,
          }
        );
      },
    }

    match reason {
      protocol::VoxelReason::Updated => {},
      protocol::VoxelReason::Requested => {
        *client.outstanding_terrain_requests.lock().unwrap() -= 1;
        debug!("Outstanding terrain requests: {}", *client.outstanding_terrain_requests.lock().unwrap());
      },
    }

    if time::precise_time_ns() - start >= 1_000_000 {
      break
    }
  }
}

#[inline(never)]
fn process_server_updates<RecvServer, UpdateView, UpdateServer, EnqueueBlockUpdates>(
  client: &client::T,
  recv_server: &mut RecvServer,
  update_view: &mut UpdateView,
  update_server: &mut UpdateServer,
  enqueue_block_updates: &mut EnqueueBlockUpdates,
) where
  RecvServer: FnMut() -> Option<protocol::ServerToClient>,
  UpdateView: FnMut(ClientToView),
  UpdateServer: FnMut(protocol::ClientToServer),
  EnqueueBlockUpdates: FnMut(Option<u64>, Vec<(voxel::bounds::T, voxel::T)>, protocol::VoxelReason),
{
  let start = time::precise_time_ns();
  let mut i = 0;
  while let Some(up) = recv_server() {
    apply_server_update(
      client,
      update_view,
      update_server,
      enqueue_block_updates,
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
