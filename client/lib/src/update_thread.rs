use cgmath;
use cgmath::Point;
use std::sync::Mutex;
use stopwatch;
use time;

use common::fnv_set;
use common::protocol;
use common::surroundings_loader;
use common::surroundings_loader::LoadType;

use audio_thread;
use block_position;
use chunk;
use client;
use edge;
use load_terrain;
use load_terrain::lod_index;
use record_book;
use server_update::apply_server_update;
use terrain_loader;
use terrain_mesh;
use view_update;
use view_update::T;
use voxel;

const MAX_OUTSTANDING_TERRAIN_REQUESTS: u32 = 1;

pub fn update_thread<RecvServer, UpdateView0, UpdateView1, UpdateAudio, UpdateServer, EnqueueTerrainUpdate>(
  quit                   : &Mutex<bool>,
  client                 : &client::T,
  recv_server            : &mut RecvServer,
  update_view0           : &mut UpdateView0,
  update_view1           : &mut UpdateView1,
  update_audio           : &mut UpdateAudio,
  update_server          : &mut UpdateServer,
  enqueue_terrain_update : &mut EnqueueTerrainUpdate,
) where
  RecvServer           : FnMut() -> Option<protocol::ServerToClient>,
  UpdateView0          : FnMut(view_update::T),
  UpdateView1          : FnMut(view_update::T),
  UpdateAudio          : FnMut(audio_thread::Message),
  UpdateServer         : FnMut(protocol::ClientToServer),
  EnqueueTerrainUpdate : FnMut(terrain_loader::Message),
{
  'update_loop: loop {
    let should_quit = *quit.lock().unwrap();
    if should_quit {
      break 'update_loop
    } else {
      stopwatch::time("update_iteration", || {
        stopwatch::time("process_server_updates", || {
          process_server_updates(client, recv_server, update_view0, update_audio, update_server, enqueue_terrain_update);
        });

        stopwatch::time("update_surroundings", || {
          update_surroundings(client, update_view1, update_server);
        });

        stopwatch::time("process_voxel_updates", || {
          process_voxel_updates(client, update_view1);
        });
      })
    }
  }
}

#[inline(never)]
fn update_surroundings<UpdateView, UpdateServer>(
  client        : &client::T,
  update_view   : &mut UpdateView,
  update_server : &mut UpdateServer,
) where
  UpdateView   : FnMut(view_update::T),
  UpdateServer : FnMut(protocol::ClientToServer),
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
    if client.pending_terrain_requests.lock().unwrap().len() as u32 >= MAX_OUTSTANDING_TERRAIN_REQUESTS {
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
    let new_lod = lod_index(distance);
    let mut requested_chunks: fnv_set::T<chunk::Position> = fnv_set::new();
    match load_type {
      LoadType::Load => {
        for edge in block_position.edges(new_lod) {
          stopwatch::time("update_thread.load_edge", || {
            let already_loaded = client.loaded_edges.lock().unwrap().contains_key(&edge);
            if already_loaded {
              debug!("Not re-loading {:?} at {:?}", block_position, new_lod);
            } else {
              let mut request_voxel = |voxel| {
                requested_chunks.insert(chunk::containing(&voxel));
              };
              load_or_request_edge(client, &mut request_voxel, update_view, &edge);
            }
          })
        }
      },
      LoadType::Update => {
        for edge in block_position.edges(new_lod) {
          stopwatch::time("update_thread.update_block", || {
            let mut request_voxel = |voxel| {
              requested_chunks.insert(chunk::containing(&voxel));
            };
            load_or_request_edge(client, &mut request_voxel, update_view, &edge);
          })
        }
      },
      LoadType::Unload => {
        let mut loaded_edges = client.loaded_edges.lock().unwrap();
        for edge in block_position.edges(new_lod) {
          stopwatch::time("update_thread.unload", || {
            // The block removal code is duplicated in load_terrain.

            loaded_edges
              .remove(&edge)
              // If it wasn't loaded, don't unload anything.
              .map(|mesh_fragment| {
                for id in &mesh_fragment.ids {
                  update_view(view_update::RemoveTerrain(*id));
                }
                for id in &mesh_fragment.grass_ids {
                  update_view(view_update::RemoveGrass(*id));
                }
              });
          })
        }
      },
    }

    for chunk in requested_chunks {
      let request_already_exists =
        !client.pending_terrain_requests
          .lock().unwrap()
          .insert(chunk);
      if !request_already_exists {
        update_server(
          protocol::ClientToServer::RequestChunk {
            requested_at : time::precise_time_ns(),
            client_id    : client.id,
            position     : chunk,
          }
        );
      }
    }

    if i >= 10 {
      i -= 10;
      if time::precise_time_ns() - start >= 1_000_000 {
        break
      }
    }
    i += 1;
  }
}

fn process_voxel_updates<UpdateView>(
  client      : &client::T,
  update_view : &mut UpdateView,
) where
  UpdateView: FnMut(view_update::T),
{
  let terrain_loader = &mut *client.terrain_loader.lock().unwrap();
  let voxels         = &mut *client.voxels.lock().unwrap();
  let rng            = &mut *client.rng.lock().unwrap();
  let id_allocator   = &mut *client.id_allocator.lock().unwrap();
  let loaded_edges   = &mut *client.loaded_edges.lock().unwrap();
  terrain_loader.tick(voxels, rng, id_allocator, loaded_edges, update_view);
}

fn load_or_request_edge<RequestVoxel, UpdateView>(
  client: &client::T,
  request_voxel: &mut RequestVoxel,
  update_view: &mut UpdateView,
  edge: &edge::T,
) where
  RequestVoxel: FnMut(voxel::bounds::T),
  UpdateView: FnMut(view_update::T),
{
  trace!("load_or_request_edge");
  match
    load_terrain::load_edge(
      client,
      update_view,
      &edge,
    )
  {
    Ok(()) => {},
    Err(()) => {
      let mut voxel_coords = Vec::new();
      let low_corner = edge.low_corner.add_v(&edge.direction.to_vec());
      voxel_coords.push(
        voxel::bounds::T {
          x: low_corner.x,
          y: low_corner.y,
          z: low_corner.z,
          lg_size: edge.lg_size,
        },
      );
      voxel_coords.extend(edge.neighbors().iter().cloned());

      for voxel in voxel_coords {
        request_voxel(voxel);
      }
    }
  }
}

#[inline(never)]
fn process_server_updates<RecvServer, UpdateView, UpdateAudio, UpdateServer, EnqueueTerrainUpdate>(
  client: &client::T,
  recv_server: &mut RecvServer,
  update_view: &mut UpdateView,
  update_audio: &mut UpdateAudio,
  update_server: &mut UpdateServer,
  enqueue_terrain_update: &mut EnqueueTerrainUpdate,
) where
  RecvServer: FnMut() -> Option<protocol::ServerToClient>,
  UpdateView: FnMut(view_update::T),
  UpdateAudio: FnMut(audio_thread::Message),
  UpdateServer: FnMut(protocol::ClientToServer),
  EnqueueTerrainUpdate: FnMut(terrain_loader::Message),
{
  let start = time::precise_time_ns();
  let mut i = 0;
  while let Some(up) = recv_server() {
    apply_server_update(
      client,
      update_view,
      update_audio,
      update_server,
      enqueue_terrain_update,
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
