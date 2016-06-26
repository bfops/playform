/// Creator of the earth.

use cgmath::{Aabb3};
use stopwatch;

use common;
use common::id_allocator;
use common::protocol;
use common::voxel;

use lod;
use server;
use terrain_loader;
use voxel_data;

#[derive(Debug, Clone, Copy)]
pub enum LoadReason {
  Local(lod::OwnerId),
  ForClient(protocol::ClientId),
}

pub enum Message {
  Load(u64, Vec<voxel::bounds::T>, LoadReason),
  Brush(voxel_data::brush::T<Box<voxel_data::mosaic::T<common::voxel::Material> + Send>>),
}

// TODO: Consider adding terrain loads to a thread pool instead of having one monolithic separate thread.
pub fn update_gaia(
  server: &server::T,
  update: Message,
) {
  stopwatch::time("update_gaia", move || {
    match update {
      Message::Load(request_time, voxel_bounds, load_reason) => {
        stopwatch::time("terrain.load", || {
          load(server, request_time, voxel_bounds, load_reason);
        });
      },
      Message::Brush(mut brush) => {
        let mut updates = Vec::new();
        server.terrain_loader.terrain.brush(
          &mut brush,
          |block, bounds| {
            trace!("update bounds {:?}", bounds);
            updates.push((*bounds, *block));
          },
        );

        let mut clients = server.clients.lock().unwrap();
        for (_, client) in clients.iter_mut() {
          client.send(
            protocol::ServerToClient::Voxels {
              requested_at : None,
              voxels       : updates.clone(),
              reason       : protocol::VoxelReason::Updated,
            }
          );
        }
      },
    };
  })
}

#[inline(never)]
fn load(
  server: &server::T,
  request_time: u64,
  voxel_bounds: Vec<voxel::bounds::T>,
  load_reason: LoadReason,
) {
  // TODO: Just lock `terrain` for the check and then the move;
  // don't lock for the whole time where we're generating the block.
  let mut lod_map = server.terrain_loader.lod_map.lock().unwrap();
  let mut in_progress_terrain = server.terrain_loader.in_progress_terrain.lock().unwrap();
  match load_reason {
    LoadReason::Local(owner) => {
      for voxel_bounds in voxel_bounds.into_iter() {
        server.terrain_loader.terrain.load(
          &voxel_bounds,
          |block| {
            let bounds =
              match block {
                &voxel::Volume(voxel::Material::Empty) => Vec::new(),
                _ => {
                  let (low, high) = voxel_bounds.corners();
                  let id = id_allocator::allocate(&server.id_allocator);
                  vec!((id, Aabb3::new(low, high)))
                },
              };
            // TODO: Check that this block isn't stale, i.e. should still be loaded.
            // Maybe this should just ping the original thread, same as we ping the client.
            terrain_loader::T::insert_block(
              &terrain_loader::LoadedTerrain { bounds: bounds },
              &voxel_bounds,
              owner,
              &server.physics,
              &mut *lod_map,
              &mut *in_progress_terrain,
              &mut *server.terrain_loader.loaded.lock().unwrap(),
            );
          }
        );
      }
    },
    LoadReason::ForClient(id) => {
      let mut voxels = Vec::new();
      for voxel_bounds in voxel_bounds.into_iter() {
        server.terrain_loader.terrain.load(
          &voxel_bounds,
          |voxel| {
            voxels.push((voxel_bounds, *voxel));
          },
        );
      }

      let mut clients = server.clients.lock().unwrap();
      let client = clients.get_mut(&id).unwrap();
      client.send(
        protocol::ServerToClient::Voxels {
          requested_at : Some(request_time),
          voxels       : voxels,
          reason       : protocol::VoxelReason::Requested,
        }
      );
    },
  }
}
