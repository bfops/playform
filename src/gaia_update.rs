use id_allocator::IdAllocator;
use lod::{LODIndex, OwnerId};
use opencl_context::CL;
use server::EntityId;
use server_update::GaiaToServer;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use stopwatch::TimerSet;
use terrain::terrain::Terrain;
use terrain::terrain_block::BlockPosition;
use terrain::texture_generator::TerrainTextureGenerator;

#[derive(Debug, Clone)]
pub enum LoadReason {
  Local(OwnerId),
  ForClient,
}

#[derive(Debug, Clone)]
pub enum ServerToGaia {
  Load(BlockPosition, LODIndex, LoadReason),
  Quit,
}

impl ServerToGaia {
  pub fn apply(
    self,
    timers: &TimerSet,
    cl: &CL,
    id_allocator: &Mutex<IdAllocator<EntityId>>,
    terrain: Arc<Mutex<Terrain>>,
    texture_generators: &[TerrainTextureGenerator],
    ups_to_server: &Sender<GaiaToServer>,
  ) -> bool {
    match self {
      ServerToGaia::Quit => {
        return false;
      },
      ServerToGaia::Load(position, lod, load_reason) => {
        timers.time("terrain.load", || {
          // TODO: Just lock `terrain` for the check and then the move,
          // not while we're generating the block.
          terrain.lock().unwrap().load(
            timers,
            cl,
            &texture_generators[lod.0 as usize],
            id_allocator,
            &position,
            lod,
            |_| {
              ups_to_server.send(
                GaiaToServer::Loaded(position, lod, load_reason)
              ).unwrap();
            },
          );
        });
      },
    }

    true
  }
}
