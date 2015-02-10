use id_allocator::IdAllocator;
use in_progress_terrain::InProgressTerrain;
use lod_map::{LOD, OwnerId, LODMap};
use noise::Seed;
use opencl_context::CL;
use physics::Physics;
use world::EntityId;
use std::sync::mpsc::Sender;
use stopwatch::TimerSet;
use terrain::terrain::Terrain;
use terrain::terrain_block::{BlockPosition, BLOCK_WIDTH};
use terrain::texture_generator::TEXTURE_WIDTH;
use terrain::texture_generator::TerrainTextureGenerator;
use view_update::ViewUpdate;
use view_update::ViewUpdate::*;

/// Load and unload TerrainBlocks from the game.
/// Each TerrainBlock can be owned by a set of owners, each of which can independently request LODs.
/// The maximum LOD requested is the one that is actually loaded.
pub struct TerrainGameLoader {
  terrain: Terrain,
  texture_generators: [TerrainTextureGenerator; 4],
  in_progress_terrain: InProgressTerrain,
  // The LODs of the currently loaded blocks.
  lod_map: LODMap,
}

impl TerrainGameLoader {
  pub fn new(cl: &CL) -> TerrainGameLoader {
    TerrainGameLoader {
      terrain: Terrain::new(Seed::new(0), 0),
      texture_generators: [
        TerrainTextureGenerator::new(cl, TEXTURE_WIDTH[0], BLOCK_WIDTH as u32),
        TerrainTextureGenerator::new(cl, TEXTURE_WIDTH[1], BLOCK_WIDTH as u32),
        TerrainTextureGenerator::new(cl, TEXTURE_WIDTH[2], BLOCK_WIDTH as u32),
        TerrainTextureGenerator::new(cl, TEXTURE_WIDTH[3], BLOCK_WIDTH as u32),
      ],
      in_progress_terrain: InProgressTerrain::new(),
      lod_map: LODMap::new(),
    }
  }

  // TODO: Disentangle the world logic from the view logic.
  // The goal is to support multiple players.

  /// Returns false if pushing into buffers fails.
  fn re_lod_block(
    &mut self,
    timers: &TimerSet,
    view: &Sender<ViewUpdate>,
    cl: &CL,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: &BlockPosition,
    loaded_lod: Option<LOD>,
    new_lod: Option<LOD>,
  ) {
    // Unload whatever's there.
    loaded_lod.map(|loaded_lod| {
      match loaded_lod {
        LOD::Placeholder => {
          self.in_progress_terrain.remove(physics, block_position);
        }
        LOD::LodIndex(loaded_lod) => {
          timers.time("terrain_game_loader.unload", || {
            let lods =
              self.terrain.all_blocks.get(block_position)
              .unwrap()
              .lods
              .as_slice();
            let block = lods[loaded_lod as usize].as_ref().unwrap();
            for id in block.ids.iter() {
              physics.remove_terrain(*id);
              view.send(RemoveTerrain(*id)).unwrap();
            }

            view.send(FreeBlock((*block_position, loaded_lod))).unwrap();
          });
        },
      };
    });

    // TODO: Avoid the double-lookup when loaded_lod and new_lod are both LodIndexes.

    // Load whatever we should be loading.
    new_lod.map(|new_lod| {
      match new_lod {
        LOD::Placeholder => {
          self.in_progress_terrain.insert(id_allocator, physics, block_position);
        },
        LOD::LodIndex(new_lod) => {
          timers.time("terrain_game_loader.load", || {
            self.terrain.load(
              timers,
              cl,
              &self.texture_generators[new_lod as usize],
              id_allocator,
              block_position,
              new_lod,
              |block| {
                timers.time("terrain_game_loader.load.physics", || {
                  for &(ref id, ref bounds) in block.bounds.iter() {
                    physics.insert_terrain(*id, bounds.clone());
                  }
                });

                timers.time("terrain_game_loader.load.gpu", || {
                  view.send(
                    PushBlock((*block_position, block.clone(), new_lod))
                  ).unwrap();
                })
              },
            )
          })
        },
      }
    });
  }

  pub fn increase_lod(
    &mut self,
    timers: &TimerSet,
    view: &Sender<ViewUpdate>,
    cl: &CL,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: &BlockPosition,
    target_lod: LOD,
    owner: OwnerId,
  ) {
    let (_, lod_change) =
      self.lod_map.increase_lod(*block_position, target_lod, owner);

    lod_change.map(|lod_change| {
      self.re_lod_block(
        timers,
        view,
        cl,
        id_allocator,
        physics,
        block_position,
        lod_change.loaded,
        lod_change.desired,
      )
    });
  }

  pub fn decrease_lod(
    &mut self,
    timers: &TimerSet,
    view: &Sender<ViewUpdate>,
    cl: &CL,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: &BlockPosition,
    target_lod: Option<LOD>,
    owner: OwnerId,
  ) {
    let (_, lod_change) =
      self.lod_map.decrease_lod(*block_position, target_lod, owner);

    lod_change.map(|lod_change| {
      self.re_lod_block(
        timers,
        view,
        cl,
        id_allocator,
        physics,
        block_position,
        lod_change.loaded,
        lod_change.desired,
      )
    });
  }
}
