use common::communicate::TerrainBlockSend;

use client::Client;
use load_terrain::load_terrain_block;
use view_update::ClientToView;

pub fn terrain_load_thread<Recv, UpdateView>(
  client: &Client,
  recv: &mut Recv,
  update_view: &mut UpdateView,
) where
  Recv: FnMut() -> Option<TerrainBlockSend>,
  UpdateView: FnMut(ClientToView),
{
  while let Some(block) = recv() {
    load_terrain_block(
      client,
      update_view,
      block,
    );
  }
}
