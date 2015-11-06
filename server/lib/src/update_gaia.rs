/// Creator of the earth.

use cgmath::{Aabb3};
use stopwatch;

use common;
use common::protocol;
use common::voxel;

use lod;
use server::Server;
use terrain_loader;
use voxel_data;

#[derive(Debug, Clone, Copy)]
pub enum LoadReason {
  Local(lod::OwnerId),
  ForClient(protocol::ClientId),
}

pub enum Message {
  Load(voxel_data::bounds::T, LoadReason),
  Brush(voxel_data::brush::T<Box<voxel_data::mosaic::T<common::voxel::Material> + Send>>),
}

// TODO: Consider adding terrain loads to a thread pool instead of having one monolithic separate thread.
pub fn update_gaia(
  server: &Server,
  update: Message,
) {
  stopwatch::time("update_gaia", move || {
    match update {
      Message::Load(position, load_reason) => {
        stopwatch::time("terrain.load", || {
          // TODO: Just lock `terrain` for the check and then the move;
          // don't lock for the whole time where we're generating the block.
          let mut lod_map = server.terrain_loader.lod_map.lock().unwrap();
          let mut in_progress_terrain = server.terrain_loader.in_progress_terrain.lock().unwrap();
          server.terrain_loader.terrain.load(
            &position,
            |block| {
              match load_reason {
                LoadReason::Local(owner) => {
                  let bounds =
                    match block {
                      &voxel::Volume(voxel::Material::Empty) => Vec::new(),
                      _ => {
                        let (low, high) = position.corners();
                        let id = server.id_allocator.lock().unwrap().allocate();
                        vec!((id, Aabb3::new(low, high)))
                      },
                    };
                  // TODO: Check that this block isn't stale, i.e. should still be loaded.
                  // Maybe this should just ping the original thread, same as we ping the client.
                  terrain_loader::T::insert_block(
                    &terrain_loader::LoadedTerrain { bounds: bounds },
                    &position,
                    owner,
                    &server.physics,
                    &mut *lod_map,
                    &mut *in_progress_terrain,
                    &mut *server.terrain_loader.loaded.lock().unwrap(),
                  );
                },
                LoadReason::ForClient(id) => {
                  let mut clients = server.clients.lock().unwrap();
                  let client = clients.get_mut(&id).unwrap();
                  client.send(
                    protocol::ServerToClient::Voxel(
                      block.clone(),
                      position,
                      protocol::VoxelReason::Requested,
                    )
                  );
                },
              }
            },
          )
        });
      },
      Message::Brush(brush) => {
        server.terrain_loader.terrain.brush(
          &brush,
          |block, position| {
            let mut clients = server.clients.lock().unwrap();
            for (_, client) in clients.iter_mut() {
              client.send(
                protocol::ServerToClient::Voxel(
                  block.clone(),
                  *position,
                  protocol::VoxelReason::Updated,
                )
              );
            }
          },
        );
      },
    };
  })
}
