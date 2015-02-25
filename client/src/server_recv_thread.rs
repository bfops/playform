use common::block_position::BlockPosition;
use common::communicate::{ServerToClient, TerrainBlockSend};

use client::Client;
use server_update::apply_server_update;
use view_update::ClientToView;

pub fn server_recv_thread<Recv, UpdateView, UpdateSurroundings, LoadTerrain>(
  client: &Client,
  recv: &mut Recv,
  update_view: &mut UpdateView,
  update_surroundings: &mut UpdateSurroundings,
  load_terrain: &mut LoadTerrain,
) where
  Recv: FnMut() -> ServerToClient,
  UpdateView: FnMut(ClientToView),
  UpdateSurroundings: FnMut(BlockPosition),
  LoadTerrain: FnMut(TerrainBlockSend),
{
  loop {
    let update = recv();
    apply_server_update(
      client,
      update_view,
      update_surroundings,
      load_terrain,
      update,
    )
  }
}
