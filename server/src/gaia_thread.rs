/// Creator of the earth.

use common::communicate::{ServerToClient, TerrainBlockSend};
use common::lod::{LODIndex, OwnerId};
use common::process_events::process_channel;
use common::stopwatch::TimerSet;
use common::block_position::BlockPosition;
use common::terrain_block::{BLOCK_WIDTH, TEXTURE_WIDTH};
use opencl_context::CL;
use server::Server;
use std::old_io::timer;
use std::ops::DerefMut;
use std::sync::mpsc::Receiver;
use std::time::duration::Duration;
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
  Quit,
}

// TODO: Consider adding terrain loads to a thread pool instead of having one monolithic separate thread.
pub fn gaia_thread(
  ups_from_server: &Receiver<ServerToGaia>,
  server: &Server,
) {
  let timers = TimerSet::new();
  let timers = &timers;

  let cl = unsafe {
    CL::new()
  };
  let cl = &cl;

  let texture_generators = [
    TerrainTextureGenerator::new(&cl, TEXTURE_WIDTH[0], BLOCK_WIDTH as u32),
    TerrainTextureGenerator::new(&cl, TEXTURE_WIDTH[1], BLOCK_WIDTH as u32),
    TerrainTextureGenerator::new(&cl, TEXTURE_WIDTH[2], BLOCK_WIDTH as u32),
    TerrainTextureGenerator::new(&cl, TEXTURE_WIDTH[3], BLOCK_WIDTH as u32),
  ];

  loop {
    let quit =
      !process_channel(
        ups_from_server,
        |update| {
          match update {
            ServerToGaia::Quit => {
              return false;
            },
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
                        server.to_client.lock().unwrap().as_mut().unwrap().send(
                          ServerToClient::AddBlock(TerrainBlockSend {
                            position: position,
                            block: block.clone(),
                            lod: lod,
                          })
                        );
                      },
                    }
                  },
                );
              });
            },
          };

          true
        }
      );
    if quit {
      break;
    }

    timer::sleep(Duration::milliseconds(1));
  }

  timers.print();
}
