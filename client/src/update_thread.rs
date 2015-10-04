use std::sync::Mutex;
use stopwatch;
use time;

use common::block_position;
use common::block_position::BlockPosition;
use common::communicate::{ClientToServer, ServerToClient, TerrainBlockSend};
use common::serialize::Copyable;
use common::surroundings_loader::LoadType;

use client;
use load_terrain::{load_terrain_block, lod_index};
use server_update::apply_server_update;
use view_update::ClientToView;

const MAX_OUTSTANDING_TERRAIN_REQUESTS: u32 = 1 << 8;

pub fn update_thread<RecvServer, RecvBlock, UpdateView0, UpdateView1, UpdateServer, QueueBlock>(
  quit: &Mutex<bool>,
  client: &client::T,
  recv_server: &mut RecvServer,
  recv_block: &mut RecvBlock,
  update_view0: &mut UpdateView0,
  update_view1: &mut UpdateView1,
  update_server: &mut UpdateServer,
  queue_block: &mut QueueBlock,
) where
  RecvServer: FnMut() -> Option<ServerToClient>,
  RecvBlock: FnMut() -> Option<TerrainBlockSend>,
  UpdateView0: FnMut(ClientToView),
  UpdateView1: FnMut(ClientToView),
  UpdateServer: FnMut(ClientToServer),
  QueueBlock: FnMut(TerrainBlockSend),
{
  'update_loop: loop {
    if *quit.lock().unwrap() == true {
      break 'update_loop
    } else {
      stopwatch::time("update_iteration", || {
        let start = time::precise_time_ns();
        while let Some(up) = recv_server() {
          apply_server_update(
            client,
            update_view0,
            update_server,
            queue_block,
            up,
          );

          if time::precise_time_ns() - start >= 1_000_000 {
            break
          }
        }

        stopwatch::time("update_surroundings", || {
          let start = time::precise_time_ns();
          let position = *client.player_position.lock().unwrap();
          let position = BlockPosition::from_world_position(&position);
          let mut loaded_blocks = client.loaded_blocks.lock().unwrap();
          let mut surroundings_loader = client.surroundings_loader.lock().unwrap();
          let mut updates =
            surroundings_loader
            .updates(position)
          ;
          loop {
            if *client.outstanding_terrain_requests.lock().unwrap() >= MAX_OUTSTANDING_TERRAIN_REQUESTS {
              break;
            }

            let block_position;
            let load_type;
            match updates.next() {
              None => break,
              Some((b, l)) => {
                block_position = b;
                load_type = l;
              },
            }
            let distance = block_position::distance(&position, &block_position);
            match load_type {
              LoadType::Load => {
                stopwatch::time("update_thread.load_block", || {
                  let lod = lod_index(distance);
                  let loaded_lod =
                    loaded_blocks
                    .get(&block_position)
                    .map(|&(_, lod)| lod);
                  if loaded_lod != Some(lod) {
                    update_server(
                      ClientToServer::RequestBlock(
                        Copyable(client.id),
                        Copyable(block_position),
                        Copyable(lod),
                      )
                    );
                    *client.outstanding_terrain_requests.lock().unwrap() += 1;
                  } else {
                    trace!("Not re-loading {:?} at {:?}", block_position, lod);
                  }
                })
              },
              LoadType::Update => {
                stopwatch::time("update_thread.update_block", || {
                  let new_lod = lod_index(distance);
                  let lod_change =
                    loaded_blocks
                    .get(&block_position)
                    .map(|&(_, lod)| new_lod < lod);
                  if lod_change == Some(true) {
                    update_server(
                      ClientToServer::RequestBlock(
                        Copyable(client.id),
                        Copyable(block_position),
                        Copyable(new_lod),
                      )
                    );
                    *client.outstanding_terrain_requests.lock().unwrap() += 1;
                  } else {
                    trace!("Not updating {:?} at {:?}", block_position, new_lod);
                  }
                })
              },
              LoadType::Unload => {
                stopwatch::time("update_thread.unload", || {
                  // The block removal code is duplicated elsewhere.

                  loaded_blocks
                    .remove(&block_position)
                    // If it wasn't loaded, don't unload anything.
                    .map(|(block, prev_lod)| {
                      for id in &block.ids {
                        update_view1(ClientToView::RemoveTerrain(*id));
                      }
                      update_view1(ClientToView::RemoveBlockData(block_position, prev_lod));
                    });
                })
              },
            };

            if time::precise_time_ns() - start >= 1_000_000 {
              break
            }
          }
        });

        let start = time::precise_time_ns();
        while let Some(block) = recv_block() {
          trace!("Got block: {:?} at {:?}", block.position, block.lod);
          load_terrain_block(
            client,
            update_view1,
            block,
          );

          if time::precise_time_ns() - start >= 1_000_000 {
            break
          }
        }
      })
    }
  }
}
