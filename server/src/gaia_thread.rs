//! Creator of the earth.

use common::stopwatch::TimerSet;
use common::terrain_block::{BLOCK_WIDTH, TEXTURE_WIDTH};

use opencl_context::CL;
use server::Server;
use terrain::texture_generator::TerrainTextureGenerator;
use update_gaia::{ServerToGaia, update_gaia};

pub fn gaia_thread<Recv>(
  server: &Server,
  recv: &mut Recv,
) where
  Recv: FnMut() -> Option<ServerToGaia>,
{
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

  while let Some(update) = recv() {
    update_gaia(
      timers,
      &server,
      &texture_generators,
      cl,
      update,
    );
  }
}
