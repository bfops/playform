use cube_shell::cube_diff;
use lod::OwnerId;
use nalgebra::Pnt3;
use player;
use std::collections::HashMap;
use surroundings_loader::SurroundingsLoader;
use terrain::terrain_block::{BlockPosition, TerrainBlock};
use terrain::terrain_vram_buffers;

pub struct Client<'a> {
  pub player_position: Pnt3<f32>,
  pub surroundings_loader: SurroundingsLoader<'a>,
  pub loaded_blocks: HashMap<BlockPosition, (TerrainBlock, u32)>,
}

impl<'a> Client<'a> {
  pub fn new(id: OwnerId) -> Client<'a> {
    let mut load_distance =
      player::Player::load_distance(terrain_vram_buffers::POLYGON_BUDGET as i32);

    // TODO: Remove this once our RAM usage doesn't skyrocket with load distance.
    let max_load_distance = 80;
    if load_distance > max_load_distance {
      info!("load_distance {} capped at {}", load_distance, max_load_distance);
      load_distance = max_load_distance;
    } else {
      info!("load_distance {}", load_distance);
    }

    Client {
      player_position: Pnt3::new(0.0, 0.0, 0.0),
      surroundings_loader:
        SurroundingsLoader::new(
          id,
          load_distance,
          Box::new(move |last, cur| {
            let mut vec = Vec::new();
            for &r in player::LOD_THRESHOLDS.iter() {
              vec.push_all(cube_diff(last, cur, r).as_slice());
            }
            vec.push_all(cube_diff(last, cur, load_distance).as_slice());
            vec
          }),
        ),
      loaded_blocks: HashMap::new(),
    }
  }
}
