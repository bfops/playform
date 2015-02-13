use gaia_update::{ServerToGaia, LoadReason};
use id_allocator::IdAllocator;
use in_progress_terrain::InProgressTerrain;
use lod::{LOD, OwnerId, LODMap};
use noise::Seed;
use physics::Physics;
use server::EntityId;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use stopwatch::TimerSet;
use terrain::terrain::Terrain;
use terrain::terrain_block::BlockPosition;

/// Load and unload TerrainBlocks from the game.
/// Each TerrainBlock can be owned by a set of owners, each of which can independently request LODs.
/// The maximum LOD requested is the one that is actually loaded.
pub struct TerrainGameLoader {
  pub terrain: Arc<Mutex<Terrain>>,
  pub in_progress_terrain: InProgressTerrain,
  pub lod_map: LODMap,
}

impl TerrainGameLoader {
  pub fn new() -> TerrainGameLoader {
    TerrainGameLoader {
      terrain: Arc::new(Mutex::new(Terrain::new(Seed::new(0), 0))),
      in_progress_terrain: InProgressTerrain::new(),
      lod_map: LODMap::new(),
    }
  }

  // TODO: Avoid the double-lookup when unload and load the same index.

  pub fn load(
    &mut self,
    timers: &TimerSet,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: &BlockPosition,
    new_lod: LOD,
    owner: OwnerId,
    ups_to_gaia: &Sender<ServerToGaia>,
  ) {

    let prev_lod;
    let max_lod_changed;
    match self.lod_map.get(block_position, owner) {
      Some((Some(prev), lods)) => {
        prev_lod = Some(prev);
        if new_lod == prev {
          return;
        }

        if new_lod < prev {
          max_lod_changed = lods.iter().filter(|&&(_, l)| l >= prev).count() < 2;
        } else {
          max_lod_changed = lods.iter().filter(|&&(_, l)| l >= new_lod).count() == 0;
        }
      },
      Some((None, lods)) => {
        max_lod_changed = lods.iter().filter(|&&(_, l)| l >= new_lod).count() == 0;
        prev_lod = None;
      },
      None => {
        max_lod_changed = true;
        prev_lod = None;
      },
    }

    if !max_lod_changed {
      // Maximum LOD is unchanged.
      let (_, change) = self.lod_map.insert(*block_position, new_lod, owner);
      assert!(change.is_none());
      return;
    }

    match new_lod {
      LOD::Placeholder => {
        let (_, change) = self.lod_map.insert(*block_position, new_lod, owner);
        let change = change.unwrap();
        assert!(change.loaded == None);
        assert!(prev_lod == None);
        assert!(change.desired == Some(LOD::Placeholder));
        self.in_progress_terrain.insert(id_allocator, physics, block_position);
      },
      LOD::LodIndex(new_lod) => {
        let terrain = self.terrain.lock().unwrap();
        match terrain.all_blocks.get(block_position) {
          None => {
            ups_to_gaia.send(
              ServerToGaia::Load(*block_position, new_lod, LoadReason::Local(owner))
            ).unwrap();
          },
          Some(mipmesh) => {
            match mipmesh.lods[new_lod.0 as usize].as_ref() {
              None => {
                debug!("{:?} requested from gaia", block_position);
                ups_to_gaia.send(
                  ServerToGaia::Load(*block_position, new_lod, LoadReason::Local(owner))
                ).unwrap();
              },
              Some(block) => {
                let new_lod = LOD::LodIndex(new_lod);
                let (_, change) =
                  self.lod_map.insert(*block_position, new_lod, owner);
                let change = change.unwrap();
                assert!(change.desired == Some(new_lod));
                let in_progress_terrain = &mut self.in_progress_terrain;
                change.loaded.map(|loaded_lod|
                  match loaded_lod {
                    LOD::Placeholder => {
                      in_progress_terrain.remove(physics, block_position);
                    }
                    LOD::LodIndex(loaded_lod) => {
                      let block = mipmesh.lods[loaded_lod.0 as usize].as_ref().unwrap();
                      timers.time("terrain_game_loader.load.unload", || {
                        for id in block.ids.iter() {
                          physics.remove_terrain(*id);
                        }
                      });
                    },
                  }
                );

                timers.time("terrain_game_loader.load.physics", || {
                  for &(ref id, ref bounds) in block.bounds.iter() {
                    physics.insert_terrain(*id, bounds.clone());
                  }
                });
              },
            }
          }
        }
      },
    };
  }

  pub fn unload(
    &mut self,
    timers: &TimerSet,
    physics: &mut Physics,
    block_position: &BlockPosition,
    owner: OwnerId,
  ) {
    let (_, mlod_change) =
      self.lod_map.remove(*block_position, owner);

    let lod_change;
    match mlod_change {
      None => {
        return;
      },
      Some(c) => lod_change = c,
    }

    lod_change.loaded.map(|loaded_lod| {
      match loaded_lod {
        LOD::Placeholder => {
          self.in_progress_terrain.remove(physics, block_position);
        }
        LOD::LodIndex(loaded_lod) => {
          timers.time("terrain_game_loader.unload", || {
            let terrain = self.terrain.lock().unwrap();
            match terrain.all_blocks.get(block_position) {
              None => {
                // Unloaded before the load request completed.
              },
              Some(block) => {
                match block.lods.get(loaded_lod.0 as usize) {
                  Some(&Some(ref block)) => {
                    for id in block.ids.iter() {
                      physics.remove_terrain(*id);
                    }
                  },
                  _ => {
                    // Unloaded before the load request completed.
                  },
                }
              },
            }
          });
        },
      }
    });
  }
}
