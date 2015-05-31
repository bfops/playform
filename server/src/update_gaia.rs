/// Creator of the earth.

use std::ops::DerefMut;
use stopwatch::TimerSet;

use common::communicate::{ClientId, ServerToClient, TerrainBlockSend};
use common::lod::{LODIndex, OwnerId};
use common::serialize::Copyable;
use common::block_position::BlockPosition;

use server::Server;
use terrain_loader::TerrainLoader;

#[derive(Debug, Clone)]
pub enum LoadReason {
  Local(OwnerId),
  ForClient(ClientId),
}

#[derive(Debug, Clone)]
pub enum ServerToGaia {
  Load(BlockPosition, LODIndex, LoadReason),
}

// TODO: Consider adding terrain loads to a thread pool instead of having one monolithic separate thread.
pub fn update_gaia(
  timers: &TimerSet,
  server: &Server,
  update: ServerToGaia,
) {
  match update {
    ServerToGaia::Load(position, lod, load_reason) => {
      timers.time("terrain.load", || {
        // TODO: Just lock `terrain` for the check and then the move;
        // don't lock for the whole time where we're generating the block.
        let mut terrain_loader = server.terrain_loader.lock().unwrap();
        let terrain_loader = terrain_loader.deref_mut();
        let lod_map = &mut terrain_loader.lod_map;
        let in_progress_terrain = &mut terrain_loader.in_progress_terrain;
        terrain_loader.terrain.load(
          timers,
          &server.id_allocator,
          &position,
          lod,
          |block| {
            match load_reason {
              LoadReason::Local(owner) => {
                // TODO: Check that this block isn't stale, i.e. should still be loaded.
                // Maybe this should just ping the original thread, same as we ping the client.
                TerrainLoader::insert_block(
                  timers,
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
          },
        );
      });
    },
  };
}
