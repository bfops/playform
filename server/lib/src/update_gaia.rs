/// Creator of the earth.

use stopwatch;

use common::communicate;
use common::communicate::{ClientId, ServerToClient, TerrainBlockSend};
use common::lod::{LODIndex, OwnerId};
use common::block_position::BlockPosition;

use server::Server;
use terrain;
use terrain_loader::TerrainLoader;
use voxel_data;

#[derive(Debug, Clone, Copy)]
pub enum LoadReason {
  Local(OwnerId),
  ForClient(ClientId),
}

pub enum Message {
  Load(BlockPosition, LODIndex, LoadReason),
  Brush(voxel_data::brush::T<Box<voxel_data::mosaic::T<terrain::voxel::Material> + Send>>),
}

// TODO: Consider adding terrain loads to a thread pool instead of having one monolithic separate thread.
pub fn update_gaia(
  server: &Server,
  update: Message,
) {
  stopwatch::time("update_gaia", move || {
    match update {
      Message::Load(position, lod, load_reason) => {
        stopwatch::time("terrain.load", || {
          // TODO: Just lock `terrain` for the check and then the move;
          // don't lock for the whole time where we're generating the block.
          let mut lod_map = server.terrain_loader.lod_map.lock().unwrap();
          let mut in_progress_terrain = server.terrain_loader.in_progress_terrain.lock().unwrap();
          server.terrain_loader.terrain.load(
            &server.id_allocator,
            &position,
            lod,
            |block| {
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
                    &mut *lod_map,
                    &mut *in_progress_terrain,
                  );
                },
                LoadReason::ForClient(id) => {
                  let mut clients = server.clients.lock().unwrap();
                  let client = clients.get_mut(&id).unwrap();
                  client.send(
                    ServerToClient::Block(
                      TerrainBlockSend {
                        position: position,
                        block: block.clone(),
                        lod: lod,
                      },
                      communicate::BlockReason::Requested,
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
          &server.id_allocator,
          &brush,
          |block, position, lod| {
            let mut clients = server.clients.lock().unwrap();
            for (_, client) in clients.iter_mut() {
              client.send(
                ServerToClient::Block(
                  TerrainBlockSend {
                    position: *position,
                    block: block.clone(),
                    lod: lod,
                  },
                  communicate::BlockReason::Updated,
                )
              );
            }
          },
        );
      },
    };
  })
}
