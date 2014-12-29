use id_allocator::IdAllocator;
use in_progress_terrain::InProgressTerrain;
use physics::Physics;
use state::EntityId;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use stopwatch::TimerSet;
use terrain::Terrain;
use terrain_block::BlockPosition;
use terrain_vram_buffers::TerrainVRAMBuffers;
use yaglw::gl_context::{GLContext, GLContextExistence};
use yaglw::shader::Shader;
use yaglw::texture::TextureUnit;

/// Load and unload TerrainBlocks from the game.
pub struct TerrainGameLoader<'a> {
  pub terrain: Terrain,
  pub terrain_vram_buffers: TerrainVRAMBuffers<'a>,
  pub in_progress_terrain: InProgressTerrain,
  // the set of blocks that are currently loaded
  pub loaded: HashSet<BlockPosition>,
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
      terrain: Terrain::new(),
      terrain_vram_buffers: terrain_vram_buffers,
      in_progress_terrain: InProgressTerrain::new(),
      loaded: HashSet::new(),
    }
  }

  pub fn load(
    &mut self,
    timers: &TimerSet,
    gl: &mut GLContext,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: &BlockPosition,
  ) -> bool {
    if !self.loaded.insert(*block_position) {
      return false;
    }

    timers.time("terrain_game_loader.load", || {
      let block = unsafe {
        self.terrain.load(timers, id_allocator, block_position)
      };

      timers.time("terrain_game_loader.load.physics", || {
        for (&id, bounds) in block.bounds.iter() {
          physics.insert_terrain(id, bounds);
        }
      });

      let terrain_vram_buffers = &mut self.terrain_vram_buffers;
      timers.time("terrain_game_loader.load.gpu", || {
        terrain_vram_buffers.push(
          gl,
          block.vertex_coordinates.as_slice(),
          block.normals.as_slice(),
          block.typs.as_slice(),
          block.ids.as_slice(),
        );
      });

      self.in_progress_terrain.remove(physics, block_position);
    });

    true
  }

  pub fn unload(
    &mut self,
    timers: &TimerSet,
    gl: &mut GLContext,
    physics: &mut Physics,
    block_position: &BlockPosition,
  ) -> bool {
    if !self.loaded.remove(block_position) {
      return false;
    }

    timers.time("terrain_game_loader.unload", || {
      let block = self.terrain.all_blocks.get(block_position).unwrap();
      for id in block.ids.iter() {
        physics.remove_terrain(*id);
        self.terrain_vram_buffers.swap_remove(gl, *id);
      }
    });

    true
  }

  /// Note that we want a specific `TerrainBlock`. Returns true if the block is not already loaded.
  pub fn mark_wanted(
    &mut self,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: &BlockPosition,
  ) -> bool {
    let r = !self.loaded.contains(block_position);
    if r {
      self.in_progress_terrain.insert(id_allocator, physics, block_position);
    }

    r
  }

  pub fn unmark_wanted(
    &mut self,
    physics: &mut Physics,
    block_position: &BlockPosition,
  ) {
    self.in_progress_terrain.remove(physics, block_position);
  }
}
