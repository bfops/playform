/// Creator of the earth.

use rustc_serialize::json;
use std::ops::DerefMut;

use common::communicate::{ServerToClient, TerrainBlockSend};
use common::lod::{LODIndex, OwnerId};
use common::stopwatch::TimerSet;
use common::block_position::BlockPosition;

use opencl_context::CL;
use server::Server;
use terrain::terrain_game_loader::TerrainGameLoader;
use terrain::texture_generator::TerrainTextureGenerator;

#[derive(Debug, Clone)]
pub enum LoadReason {
  Local(OwnerId),
  ForClient,
}

#[derive(Debug, Clone)]
pub enum ServerToGaia {
  Load(BlockPosition, LODIndex, LoadReason),
}

// TODO: Consider adding terrain loads to a thread pool instead of having one monolithic separate thread.
pub fn update_gaia(
  timers: &TimerSet,
  server: &Server,
  texture_generators: &[TerrainTextureGenerator],
  cl: &CL,
  update: ServerToGaia,
) {
  match update {
    ServerToGaia::Load(position, lod, load_reason) => {
      timers.time("terrain.load", || {
        // TODO: Just lock `terrain` for the check and then the move,
        // not while we're generating the block.
        let mut terrain_game_loader = server.terrain_game_loader.lock().unwrap();
        let terrain_game_loader = terrain_game_loader.deref_mut();
        let lod_map = &mut terrain_game_loader.lod_map;
        let in_progress_terrain = &mut terrain_game_loader.in_progress_terrain;
        terrain_game_loader.terrain.load(
          timers,
          cl,
          &texture_generators[lod.0 as usize],
          &server.id_allocator,
          &position,
          lod,
          |block| {
            match load_reason {
              LoadReason::Local(owner) => {
                // TODO: Check that this block isn't stale; i.e. should still be loaded.
                // Maybe this should just ping the original thread, same as we ping the client.
                TerrainGameLoader::insert_block(
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
              LoadReason::ForClient => {
                let mut to_client = server.to_client.lock().unwrap();
                let &mut (ref mut to_client, _) = to_client.as_mut().unwrap();
                to_client.send(Some(
                  ServerToClient::AddBlock(TerrainBlockSend {
                    position: position,
                    block: json::encode(&block).unwrap(),
                    lod: lod,
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
