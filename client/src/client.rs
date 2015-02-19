//! Main Playform client state code.

use common::block_position::BlockPosition;
use common::lod::LODIndex;
use common::terrain_block::TerrainBlock;
use nalgebra::Pnt3;
use std::collections::HashMap;
use std::sync::Mutex;

/// The distances at which LOD switches.
pub const LOD_THRESHOLDS: [i32; 3] = [1, 8, 32];

/// The main client state.
pub struct Client<'a> {
  #[allow(missing_docs)]
  pub player_position: Mutex<Pnt3<f32>>,
  /// A record of all the blocks that have been loaded.
  pub loaded_blocks: Mutex<HashMap<BlockPosition, (TerrainBlock, LODIndex)>>,
}

impl<'a> Client<'a> {
  #[allow(missing_docs)]
  pub fn new() -> Client<'a> {
    Client {
      player_position: Mutex::new(Pnt3::new(0.0, 0.0, 0.0)),
      loaded_blocks: Mutex::new(HashMap::new()),
    }
  }
}
