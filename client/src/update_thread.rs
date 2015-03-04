use std::sync::Mutex;
use std::thread;

use common::communicate::{ClientToServer, ServerToClient, TerrainBlockSend};

use client::Client;
use load_terrain::load_terrain_block;
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
    } else if let Some(block) = recv_block() {
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
