use std::sync::Mutex;
use std::thread;
use stopwatch;
use time;

use common::block_position::BlockPosition;
use common::communicate::{ClientToServer, ServerToClient, TerrainBlockSend};
use common::interval_timer::IntervalTimer;
use common::serialize::Copyable;
use common::surroundings_loader::LODChange;

use client::Client;
use load_terrain::{load_terrain_block, lod_index};
use server_update::apply_server_update;
use view_update::ClientToView;

pub fn update_thread<RecvServer, RecvBlock, UpdateView, UpdateServer, QueueBlock>(
  quit: &Mutex<bool>,
  client: &Client,
  recv_server: &mut RecvServer,
  recv_block: &mut RecvBlock,
  update_view: &mut UpdateView,
  update_server: &mut UpdateServer,
  queue_block: &mut QueueBlock,
) where
  RecvServer: FnMut() -> Option<ServerToClient>,
  RecvBlock: FnMut() -> Option<TerrainBlockSend>,
  UpdateView: FnMut(ClientToView),
  UpdateServer: FnMut(ClientToServer),
  QueueBlock: FnMut(TerrainBlockSend),
{
  let mut surroundings_timer = IntervalTimer::new(500_000_000, time::precise_time_ns());

  'update_loop: loop {
    if *quit.lock().unwrap() == true {
      break 'update_loop;
    } else if let Some(up) = recv_server() {
      apply_server_update(
        client,
        update_view,
        update_server,
        queue_block,
        up,
      );
    } else {
      if surroundings_timer.update(time::precise_time_ns()) > 0 {
        stopwatch::time("update_surroundings", || {
          let position = *client.player_position.lock().unwrap();
          let position = BlockPosition::from_world_position(&position);
          let mut loaded_blocks = client.loaded_blocks.lock().unwrap();
          let mut surroundings_loader = client.surroundings_loader.lock().unwrap();
          let updates =
            surroundings_loader
            .updates(position)
            .take(1 << 16)
          ;
          for lod_change in updates {
            match lod_change {
              LODChange::Load(block_position, distance) => {
                stopwatch::time("update_thread::load_block", || {
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
                  } else {
                    trace!("Not re-loading {:?} at {:?}", block_position, lod);
                  }
                })
              },
              LODChange::Unload(block_position) => {
                stopwatch::time("update_thread::unload", || {
                  // The block removal code is duplicated elsewhere.

                  loaded_blocks
                    .remove(&block_position)
                    // If it wasn't loaded, don't unload anything.
                    .map(|(block, prev_lod)| {
                      for id in &block.ids {
                        update_view(ClientToView::RemoveTerrain(*id));
                      }
                      update_view(ClientToView::RemoveBlockData(block_position, prev_lod));
                    });
                })
              },
            };
          }
        });
      } else if let Some(block) = recv_block() {
        trace!("Got block: {:?} at {:?}", block.position, block.lod);
        load_terrain_block(
          client,
          update_view,
          block,
        );
      } else {
        thread::yield_now();
      }
    }
  }
}
