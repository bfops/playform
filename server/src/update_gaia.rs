/// Creator of the earth.

use std::ops::DerefMut;
use stopwatch;

use common::communicate::{ClientId, ServerToClient, TerrainBlockSend};
use common::lod::{LODIndex, OwnerId};
use common::serialize::Copyable;
use common::block_position::BlockPosition;

use server::Server;
use terrain_loader::TerrainLoader;
use voxel;

#[derive(Debug, Clone, Copy)]
pub enum LoadReason {
  Local(OwnerId),
  ForClient(ClientId),
}

pub enum ServerToGaia {
  Load(BlockPosition, LODIndex, LoadReason),
  Brush(voxel::brush::T<Box<voxel::mosaic::T + Send>>),
}

// TODO: Consider adding terrain loads to a thread pool instead of having one monolithic separate thread.
pub fn update_gaia(
  server: &Server,
  update: ServerToGaia,
) {
  match update {
    ServerToGaia::Load(position, lod, load_reason) => {
      stopwatch::time("terrain.load", || {
        // TODO: Just lock `terrain` for the check and then the move;
        // don't lock for the whole time where we're generating the block.
        let mut terrain_loader = server.terrain_loader.lock().unwrap();
        let terrain_loader = terrain_loader.deref_mut();
        let lod_map = &mut terrain_loader.lod_map;
        let in_progress_terrain = &mut terrain_loader.in_progress_terrain;
        let block =
          terrain_loader.terrain.load(
            &server.id_allocator,
            &position,
            lod,
          );

        match load_reason {
          LoadReason::Local(owner) => {
            // TODO: Check that this block isn't stale, i.e. should still be loaded.
            // Maybe this should just ping the original thread, same as we ping the client.
            TerrainLoader::insert_block(
              block,
              &position,
              lod,
              owner,
              &server.physics,
              lod_map,
              in_progress_terrain,
            );
          },
          LoadReason::ForClient(id) => {
            let clients = server.clients.lock().unwrap();
            let client = clients.get(&id).unwrap();
            client.sender.send(Some(
              ServerToClient::UpdateBlock(TerrainBlockSend {
                position: Copyable(position),
                block: block.clone(),
                lod: Copyable(lod),
              })
            )).unwrap();
          },
        }
      });
    },
    ServerToGaia::Brush(brush) => {
      let mut terrain_loader = server.terrain_loader.lock().unwrap();
      terrain_loader.terrain.brush(
        &server.id_allocator,
        &brush,
        |block, position, lod| {
          let clients = server.clients.lock().unwrap();
          for client in clients.values() {
            client.sender.send(Some(
              ServerToClient::UpdateBlock(TerrainBlockSend {
                position: Copyable(*position),
                block: block.clone(),
                lod: Copyable(lod),
              })
            )).unwrap();
          }
        },
      );
    },
  };
}
