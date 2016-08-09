/// Creator of the earth.

use collision::{Aabb3};
use stopwatch;

use common;
use common::id_allocator;
use common::protocol;
use common::voxel;

use lod;
use server;
use terrain_loader;
use terrain::chunk;
use voxel_data;

#[derive(Debug, Clone, Copy)]
pub enum LoadDestination {
  Local(lod::OwnerId),
  Client(protocol::ClientId),
}

pub enum Message {
  LoadChunk {
    request_time_ns : u64,
    position        : chunk::position::T,
    lg_voxel_size   : i16,
    destination     : LoadDestination,
  },
  LoadVoxel {
    bounds      : voxel::bounds::T,
    destination : LoadDestination,
  },
  Brush(voxel_data::brush::T<Box<voxel_data::mosaic::T<common::voxel::Material> + Send>>),
}

// TODO: Consider adding terrain loads to a thread pool instead of having one monolithic separate thread.
pub fn update_gaia(
  server: &server::T,
  update: Message,
) {
  stopwatch::time("update_gaia", move || {
    match update {
      Message::LoadChunk { request_time_ns, position, lg_voxel_size, destination } => {
        stopwatch::time("terrain.load_chunk", || {
          load_chunk(server, request_time_ns, position, lg_voxel_size, destination);
        });
      },
      Message::LoadVoxel { bounds, destination } => {
        stopwatch::time("terrain.load_voxel", || {
          load_voxel(server, bounds, destination);
        });
      },
      Message::Brush(mut brush) => {
        let mut voxels = Vec::new();
        server.terrain_loader.terrain.brush(
          &mut brush,
          |block, bounds| {
            trace!("update bounds {:?}", bounds);
            voxels.push((*bounds, *block));
          },
        );

        let mut clients = server.clients.lock().unwrap();
        for (_, client) in clients.iter_mut() {
          client.send(
            protocol::ServerToClient::Voxels { voxels: voxels.clone() },
          );
        }
      },
    };
  })
}

#[inline(never)]
fn load_chunk(
  server        : &server::T,
  request_time_ns  : u64,
  position      : chunk::position::T,
  lg_voxel_size : i16,
  destination   : LoadDestination,
) {
  // TODO: Just lock `terrain` for the check and then the move;
  // don't lock for the whole time where we're generating the block.
  let mut lod_map = server.terrain_loader.lod_map.lock().unwrap();
  let mut in_progress_terrain = server.terrain_loader.in_progress_terrain.lock().unwrap();
  match destination {
    LoadDestination::Local(owner) => {
      for voxel_bounds in chunk::voxel_bounds(&position, lg_voxel_size) {
        let voxel = server.terrain_loader.terrain.load(&voxel_bounds);
        let bounds =
          match voxel {
            voxel::Volume(voxel::Material::Empty) => Vec::new(),
            voxel::Surface(_) | voxel::Volume(_) => {
              let (low, high) = voxel_bounds.corners();
              let id = id_allocator::allocate(&server.id_allocator);
              vec!((id, Aabb3::new(low, high)))
            },
          };
        // TODO: Check that this voxel isn't stale, i.e. should still be loaded.
        // Maybe this should just ping the original thread, same as we ping the client.
        terrain_loader::T::insert_voxel(
          &terrain_loader::LoadedTerrain { bounds: bounds },
          &voxel_bounds,
          owner,
          &server.physics,
          &mut *lod_map,
          &mut *in_progress_terrain,
          &mut *server.terrain_loader.loaded.lock().unwrap(),
        );
      }
    },
    LoadDestination::Client(id) => {
      let chunk =
        chunk::of_callback(
          &position,
          lg_voxel_size,
          |bounds| { server.terrain_loader.terrain.load(&bounds) },
        );

      let mut clients = server.clients.lock().unwrap();
      let client = clients.get_mut(&id).unwrap();
      client.send(
        protocol::ServerToClient::Chunk {
          request_time_ns : request_time_ns,
          chunk           : chunk,
          position        : position,
          lg_voxel_size   : lg_voxel_size,
        }
      );
    },
  }
}

fn load_voxel(
  server       : &server::T,
  voxel_bounds : voxel::bounds::T,
  destination  : LoadDestination,
) {
  // TODO: Just lock `terrain` for the check and then the move;
  // don't lock for the whole time where we're generating the block.
  let mut lod_map = server.terrain_loader.lod_map.lock().unwrap();
  let mut in_progress_terrain = server.terrain_loader.in_progress_terrain.lock().unwrap();
  match destination {
    LoadDestination::Local(owner) => {
      let bounds =
        match server.terrain_loader.terrain.load(&voxel_bounds) {
          voxel::Volume(voxel::Material::Empty) => Vec::new(),
          voxel::Surface(_) | voxel::Volume(_) => {
            let (low, high) = voxel_bounds.corners();
            let id = id_allocator::allocate(&server.id_allocator);
            vec!((id, Aabb3::new(low, high)))
          },
        };
      // TODO: Check that this voxel isn't stale, i.e. should still be loaded.
      // Maybe this should just ping the original thread, same as we ping the client.
      terrain_loader::T::insert_voxel(
        &terrain_loader::LoadedTerrain { bounds: bounds },
        &voxel_bounds,
        owner,
        &server.physics,
        &mut *lod_map,
        &mut *in_progress_terrain,
        &mut *server.terrain_loader.loaded.lock().unwrap(),
      );
    },
    LoadDestination::Client(id) => {
      let voxel = server.terrain_loader.terrain.load(&voxel_bounds);
      let mut clients = server.clients.lock().unwrap();
      let client = clients.get_mut(&id).unwrap();
      client.send(
        protocol::ServerToClient::Voxels { voxels: vec!((voxel_bounds, voxel)) }
      );
    },
  }
}
