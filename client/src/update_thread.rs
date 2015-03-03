use std::thread;
use time;

use common::communicate::{ClientToServer, ServerToClient, TerrainBlockSend};

use client::Client;
use load_terrain::load_terrain_block;
use server_update::apply_server_update;
use view_update::ClientToView;

pub fn update_thread<RecvServer, RecvBlock, UpdateView, UpdateServer, QueueBlock>(
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
  // TODO: quit?
  'update_loop: loop {
    if let Some(up) = recv_server() {
      println!("{} server update: {:?}", time::precise_time_ns(), up);
      apply_server_update(
        client,
        update_view,
        update_server,
        queue_block,
        up,
      );
    } else {
      if let Some(block) = recv_block() {
        println!("{} block update", time::precise_time_ns());
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
