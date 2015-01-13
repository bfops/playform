use id_allocator::IdAllocator;
use in_progress_terrain::InProgressTerrain;
use lod_map::{LOD, OwnerId, LODMap};
use noise::Seed;
use physics::Physics;
use state::EntityId;
use std::cell::RefCell;
use std::rc::Rc;
use stopwatch::TimerSet;
use terrain::Terrain;
use terrain_block::BlockPosition;
use terrain_vram_buffers::TerrainVRAMBuffers;
use yaglw::gl_context::{GLContext, GLContextExistence};
use yaglw::shader::Shader;
use yaglw::texture::TextureUnit;

/// Load and unload TerrainBlocks from the game.
/// Each TerrainBlock can be owned by a set of owners, each of which can independently request LODs.
/// The maximum LOD requested is the one that is actually loaded.
pub struct TerrainGameLoader<'a> {
  pub terrain: Terrain,
  pub terrain_vram_buffers: TerrainVRAMBuffers<'a>,
  pub in_progress_terrain: InProgressTerrain,
  // The blocks that are currently loaded, and their owners and 
  pub lod_map: LODMap,
}

impl<'a> TerrainGameLoader<'a> {
  pub fn new(
    gl: &'a GLContextExistence,
    gl_context: &mut GLContext,
    shader: Rc<RefCell<Shader>>,
    texture_unit_alloc: &mut IdAllocator<TextureUnit>,
  ) -> TerrainGameLoader<'a> {
    let terrain_vram_buffers = TerrainVRAMBuffers::new(gl, gl_context);
    terrain_vram_buffers.bind_glsl_uniforms(gl_context, texture_unit_alloc, shader.clone());

    TerrainGameLoader {
      terrain: Terrain::new(Seed::new(0)),
      terrain_vram_buffers: terrain_vram_buffers,
      in_progress_terrain: InProgressTerrain::new(),
      lod_map: LODMap::new(),
    }
  }

  fn re_lod_block(
    &mut self,
    timers: &TimerSet,
    gl: &mut GLContext,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: &BlockPosition,
    loaded_lod: Option<LOD>,
    new_lod: Option<LOD>,
  ) {
    // Unload whatever's there.
    match loaded_lod {
      None => {},
      Some(LOD::Placeholder) => {
        self.in_progress_terrain.remove(physics, block_position);
      }
      Some(LOD::LodIndex(loaded_lod)) => {
        timers.time("terrain_game_loader.unload", || {
          let lods =
            self.terrain.all_blocks.get(block_position)
            .unwrap()
            .lods
            .as_slice();
          let block = lods[loaded_lod as usize].as_ref().unwrap();
          for id in block.ids.iter() {
            physics.remove_terrain(*id);
            self.terrain_vram_buffers.swap_remove(gl, *id);
          }
        });
      },
    }

    // TODO: Avoid the double-lookup when loaded_lod and new_lod are both LodIndexes.

    // Load whatever we should be loading.
    match new_lod {
      None => {},
      Some(LOD::Placeholder) => {
        self.in_progress_terrain.insert(id_allocator, physics, block_position);
      },
      Some(LOD::LodIndex(new_lod)) => {
        timers.time("terrain_game_loader.load", || {
          let block = unsafe {
            self.terrain.load(timers, id_allocator, block_position, new_lod)
          };
    
          timers.time("terrain_game_loader.load.physics", || {
            for (&id, bounds) in block.bounds.iter() {
              physics.insert_terrain(id, bounds.clone());
            }
          });
    
          let terrain_vram_buffers = &mut self.terrain_vram_buffers;
          timers.time("terrain_game_loader.load.gpu", || {
            terrain_vram_buffers.push(
              gl,
              block.vertex_coordinates.as_slice(),
              block.normals.as_slice(),
              block.colors.as_slice(),
              block.ids.as_slice(),
            );
          });
        });
      },
    };
  }

  pub fn increase_lod(
    &mut self,
    timers: &TimerSet,
    gl: &mut GLContext,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: &BlockPosition,
    target_lod: LOD,
    owner: OwnerId,
  ) {
    self.lod_map
      .increase_lod(*block_position, target_lod, owner)
      .map(|lod_change|
        self.re_lod_block(
          timers,
          gl,
          id_allocator,
          physics,
          block_position,
          lod_change.loaded,
          lod_change.desired,
        ));
  }

  pub fn decrease_lod(
    &mut self,
    timers: &TimerSet,
    gl: &mut GLContext,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: &BlockPosition,
    target_lod: Option<LOD>,
    owner: OwnerId,
  ) {
    self.lod_map
      .decrease_lod(*block_position, target_lod, owner)
      .map(|lod_change|
        self.re_lod_block(
          timers,
          gl,
          id_allocator,
          physics,
          block_position,
          lod_change.loaded,
          lod_change.desired,
        ));
  }
}
