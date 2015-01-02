use id_allocator::IdAllocator;
use in_progress_terrain::InProgressTerrain;
use physics::Physics;
use state::EntityId;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::rc::Rc;
use stopwatch::TimerSet;
use terrain::Terrain;
use terrain_block::BlockPosition;
use terrain_vram_buffers::TerrainVRAMBuffers;
use yaglw::gl_context::{GLContext, GLContextExistence};
use yaglw::shader::Shader;
use yaglw::texture::TextureUnit;

/// These are used to identify the owners of terrain load operations.
#[deriving(Copy, Clone, Show, PartialEq, Eq, Hash, Default)]
pub struct OwnerId(u32);

impl Add<u32, OwnerId> for OwnerId {
  fn add(self, rhs: u32) -> OwnerId {
    let OwnerId(id) = self;
    OwnerId(id + rhs)
  }
}

/// Load and unload TerrainBlocks from the game.
pub trait TerrainGameLoader {
  /// Ensure a `TerrainBlock` is loaded.
  fn load(
    &mut self,
    timers: &TimerSet,
    gl: &mut GLContext,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: &BlockPosition,
    lod: uint,
    owner: OwnerId,
  ) -> bool;

  /// Release a request for a `TerrainBlock`.
  fn unload(
    &mut self,
    timers: &TimerSet,
    gl: &mut GLContext,
    physics: &mut Physics,
    block_position: &BlockPosition,
    owner: OwnerId,
  ) -> bool;

  /// If the block is not already loaded, insert a solid block as a placeholder.
  /// Returns true if the block is not already loaded.
  fn insert_placeholder(
    &mut self,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: &BlockPosition,
    owner: OwnerId,
  ) -> bool;

  /// Remove a placeholder that was inserted by insert_placeholder.
  /// Returns true if a placeholder was removed.
  fn remove_placeholder(
    &mut self,
    physics: &mut Physics,
    block_position: &BlockPosition,
    owner: OwnerId,
  ) -> bool;
}

struct BlockLoadState {
  pub owners: HashSet<OwnerId>,
  /// If this is None, only a placeholder is loaded.
  pub loaded_lod: Option<uint>,
}

pub struct Default<'a> {
  pub terrain: Terrain,
  pub terrain_vram_buffers: TerrainVRAMBuffers<'a>,
  pub in_progress_terrain: InProgressTerrain,
  // The blocks that are currently loaded, and their owners and 
  pub loaded: HashMap<BlockPosition, BlockLoadState>,
}

impl<'a> Default<'a> {
  pub fn new(
    gl: &'a GLContextExistence,
    gl_context: &mut GLContext,
    shader: Rc<RefCell<Shader>>,
    texture_unit_alloc: &mut IdAllocator<TextureUnit>,
  ) -> Default<'a> {
    let terrain_vram_buffers = TerrainVRAMBuffers::new(gl, gl_context);
    terrain_vram_buffers.bind_glsl_uniforms(gl_context, texture_unit_alloc, shader.clone());

    Default {
      terrain: Terrain::new(),
      terrain_vram_buffers: terrain_vram_buffers,
      in_progress_terrain: InProgressTerrain::new(),
      loaded: HashMap::new(),
    }
  }
}

impl<'a> TerrainGameLoader for Default<'a> {
  fn load(
    &mut self,
    timers: &TimerSet,
    gl: &mut GLContext,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: &BlockPosition,
    lod: uint,
    owner: OwnerId,
  ) -> bool {
    match self.loaded.entry(*block_position) {
      Entry::Occupied(mut entry) => {
        let block_load_state = entry.get_mut();
        let already_loaded = block_load_state.loaded_lod.is_some();
        block_load_state.owners.insert(owner);
        if already_loaded {
          return false;
        }
        block_load_state.loaded_lod = Some(lod);
      },
      Entry::Vacant(entry) => {
        let mut owners = HashSet::new();
        owners.insert(owner);
        entry.set(BlockLoadState {
          owners: owners,
          loaded_lod: Some(lod),
        });
      },
    }

    timers.time("terrain_game_loader.load", || {
      let block = unsafe {
        self.terrain.load(timers, id_allocator, block_position, lod)
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
          block.typs.as_slice(),
          block.ids.as_slice(),
        );
      });

      self.in_progress_terrain.remove(physics, block_position);
    });

    true
  }

  fn unload(
    &mut self,
    timers: &TimerSet,
    gl: &mut GLContext,
    physics: &mut Physics,
    block_position: &BlockPosition,
    owner: OwnerId,
  ) -> bool {
    let loaded_lod;
    match self.loaded.entry(*block_position) {
      Entry::Occupied(mut entry) => {
        {
          let block_load_state = entry.get_mut();
          match block_load_state.loaded_lod {
            None => return false,
            Some(lod) => loaded_lod = lod,
          };
          let should_unload =
            block_load_state.owners.remove(&owner)
            && block_load_state.owners.is_empty();
          if !should_unload {
            return false;
          }
        }
        entry.take();
      },
      Entry::Vacant(_) => {
        return false;
      },
    };

    timers.time("terrain_game_loader.unload", || {
      let block = &self.terrain.all_blocks.get(block_position).unwrap().lods[loaded_lod];
      for id in block.ids.iter() {
        physics.remove_terrain(*id);
        self.terrain_vram_buffers.swap_remove(gl, *id);
      }
    });

    true
  }

  /// Note that we want a specific `TerrainBlock`.
  fn insert_placeholder(
    &mut self,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: &BlockPosition,
    owner: OwnerId,
  ) -> bool {
    match self.loaded.entry(*block_position) {
      Entry::Occupied(mut entry) => {
        let block_load_state = entry.get_mut();
        block_load_state.owners.insert(owner);
        false
      },
      Entry::Vacant(entry) => {
        let mut owners = HashSet::new();
        owners.insert(owner);
        entry.set(BlockLoadState {
          owners: owners,
          loaded_lod: None,
        });
        assert!(self.in_progress_terrain.insert(id_allocator, physics, block_position));
        true
      },
    }
  }

  fn remove_placeholder(
    &mut self,
    physics: &mut Physics,
    block_position: &BlockPosition,
    owner: OwnerId,
  ) -> bool {
    match self.loaded.entry(*block_position) {
      Entry::Occupied(mut entry) => {
        {
          let block_load_state = entry.get_mut();
          let should_unload =
            block_load_state.loaded_lod.is_none()
            && block_load_state.owners.remove(&owner)
            && block_load_state.owners.is_empty();
          if !should_unload {
            return false;
          }
        }
        entry.take();
      },
      Entry::Vacant(_) => {
        return false;
      },
    };

    assert!(self.in_progress_terrain.remove(physics, block_position));

    true
  }
}
