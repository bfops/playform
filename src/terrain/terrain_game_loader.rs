use id_allocator::IdAllocator;
use in_progress_terrain::InProgressTerrain;
use lod::{LOD, OwnerId, LODMap};
use noise::Seed;
use opencl_context::CL;
use physics::Physics;
use server::EntityId;
use stopwatch::TimerSet;
use terrain::terrain::Terrain;
use terrain::terrain_block::{BlockPosition, BLOCK_WIDTH};
use terrain::texture_generator::TEXTURE_WIDTH;
use terrain::texture_generator::TerrainTextureGenerator;

/// Load and unload TerrainBlocks from the game.
/// Each TerrainBlock can be owned by a set of owners, each of which can independently request LODs.
/// The maximum LOD requested is the one that is actually loaded.
pub struct TerrainGameLoader {
  pub terrain: Terrain,
  pub texture_generators: [TerrainTextureGenerator; 4],
  pub in_progress_terrain: InProgressTerrain,
  // The LODs of the currently loaded blocks.
  pub lod_map: LODMap,
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

  // TODO: Avoid the double-lookup when unload and load the same index.

  pub fn load(
    &mut self,
    timers: &TimerSet,
    cl: &CL,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: &BlockPosition,
    target_lod: LOD,
    owner: OwnerId,
  ) {
    let (_, mlod_change) =
      self.lod_map.increase_lod(*block_position, target_lod, owner);

    let lod_change;
    match mlod_change {
      None => {
        return;
      },
      Some(c) => lod_change = c,
    }

    lod_change.loaded.map(|loaded_lod| {
      self.unload_loaded(
        timers,
        physics,
        block_position,
        loaded_lod,
      );
    });

    let target_lod = lod_change.desired.unwrap();

    match target_lod {
      LOD::Placeholder => {
        self.in_progress_terrain.insert(id_allocator, physics, block_position);
      },
      LOD::LodIndex(target_lod) => {
        timers.time("terrain_game_loader.load", || {
          self.terrain.load(
            timers,
            cl,
            &self.texture_generators[target_lod.0 as usize],
            id_allocator,
            block_position,
            target_lod,
            |block| {
              timers.time("terrain_game_loader.load.physics", || {
                for &(ref id, ref bounds) in block.bounds.iter() {
                  physics.insert_terrain(*id, bounds.clone());
                }
              });
            },
          )
        })
      },
    }
  }

  fn unload_loaded(
    &mut self,
    timers: &TimerSet,
    physics: &mut Physics,
    block_position: &BlockPosition,
    loaded: LOD,
  ) {
    match loaded {
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
          let block = lods[loaded_lod.0 as usize].as_ref().unwrap();
          for id in block.ids.iter() {
            physics.remove_terrain(*id);
          }
        });
      },
    }
  }

  pub fn unload(
    &mut self,
    timers: &TimerSet,
    physics: &mut Physics,
    block_position: &BlockPosition,
    owner: OwnerId,
  ) {
    let (_, mlod_change) =
      self.lod_map.decrease_lod(*block_position, None, owner);

    let lod_change;
    match mlod_change {
      None => {
        return;
      },
      Some(c) => lod_change = c,
    }

    lod_change.loaded.map(|loaded_lod| {
      self.unload_loaded(
        timers,
        physics,
        block_position,
        loaded_lod,
      );
    });
  }
}
