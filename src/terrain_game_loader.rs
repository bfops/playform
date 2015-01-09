use id_allocator::IdAllocator;
use in_progress_terrain::InProgressTerrain;
use noise::Seed;
use physics::Physics;
use state::EntityId;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::ops::Add;
use std::rc::Rc;
use stopwatch::TimerSet;
use terrain::Terrain;
use terrain_block::BlockPosition;
use terrain_vram_buffers::TerrainVRAMBuffers;
use yaglw::gl_context::{GLContext, GLContextExistence};
use yaglw::shader::Shader;
use yaglw::texture::TextureUnit;

/// These are used to identify the owners of terrain load operations.
#[derive(Copy, Clone, Show, PartialEq, Eq, Hash, Default)]
pub struct OwnerId(u32);

impl Add<u32> for OwnerId {
  type Output = OwnerId;

  fn add(self, rhs: u32) -> OwnerId {
    let OwnerId(id) = self;
    OwnerId(id + rhs)
  }
}

#[derive(Show, Clone, Copy, PartialEq, Eq)]
pub enum LOD {
  LodIndex(uint),
  // An invisible solid block
  Placeholder,
}

impl PartialOrd for LOD {
  #[inline(always)]
  fn partial_cmp(&self, other: &LOD) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for LOD {
  #[inline(always)]
  fn cmp(&self, other: &LOD) -> Ordering {
    match (*self, *other) {
      (LOD::Placeholder, LOD::Placeholder) => Ordering::Equal,
      (LOD::Placeholder, LOD::LodIndex(_)) => Ordering::Less,
      (LOD::LodIndex(_), LOD::Placeholder) => Ordering::Greater,
      (LOD::LodIndex(idx1), LOD::LodIndex(idx2)) =>
        // A greater level of detail is a lower index, so invert the result of the index comparison.
        match idx1.cmp(&idx2) {
          Ordering::Less => Ordering::Greater,
          Ordering::Greater => Ordering::Less,
          ord => ord,
        }
    }
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
    lod_index: LOD,
    owner: OwnerId,
  );

  /// Like `load`, but only runs if the requested LOD is lower than the current one.
  fn decrease_lod(
    &mut self,
    timers: &TimerSet,
    gl: &mut GLContext,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: &BlockPosition,
    requested_lod: LOD,
    owner: OwnerId,
  );

  /// Release a request for a `TerrainBlock`.
  fn unload(
    &mut self,
    timers: &TimerSet,
    gl: &mut GLContext,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: &BlockPosition,
    owner: OwnerId,
  );

  /// If the block is not already loaded, insert a solid block as a placeholder.
  /// Returns true if the block is not already loaded.
  fn insert_placeholder(
    &mut self,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: &BlockPosition,
    owner: OwnerId,
  );

  /// Remove a placeholder that was inserted by insert_placeholder.
  /// Returns true if a placeholder was removed.
  fn remove_placeholder(
    &mut self,
    physics: &mut Physics,
    block_position: &BlockPosition,
    owner: OwnerId,
  );
}

struct BlockLoadState {
  /// The LOD indexes requested by each owner of this block.
  pub owner_lods: HashMap<OwnerId, LOD>,
  pub loaded_lod: LOD,
}

/// Load and unload TerrainBlocks from the game.
/// Each TerrainBlock can be owned by a set of owners, each of which can independently request LODs.
/// The maximum LOD requested is the one that is actually loaded.
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
      terrain: Terrain::new(Seed::new(0)),
      terrain_vram_buffers: terrain_vram_buffers,
      in_progress_terrain: InProgressTerrain::new(),
      loaded: HashMap::new(),
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
          let block = lods[loaded_lod].as_ref().unwrap();
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
              block.typs.as_slice(),
              block.ids.as_slice(),
            );
          });
        });
      },
    };
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
    requested_lod: LOD,
    owner: OwnerId,
  ) {
    let loaded_lod;
    let new_lod;
    match self.loaded.entry(block_position) {
      Entry::Vacant(entry) => {
        let mut owner_lods = HashMap::new();
        owner_lods.insert(owner, requested_lod);
        entry.insert(BlockLoadState {
          owner_lods: owner_lods,
          loaded_lod: requested_lod,
        });

        loaded_lod = None;
        new_lod = requested_lod;
      },
      Entry::Occupied(mut entry) => {
        let block_load_state = entry.get_mut();

        block_load_state.owner_lods.insert(owner, requested_lod);
        new_lod = *block_load_state.owner_lods.values().max_by(|x| *x).unwrap();

        if block_load_state.loaded_lod == new_lod {
          // Already loaded at the right LOD.
          return;
        }

        loaded_lod = Some(block_load_state.loaded_lod);
        block_load_state.loaded_lod = new_lod;
      },
    }

    self.re_lod_block(timers, gl, id_allocator, physics, block_position, loaded_lod, Some(new_lod));
  }

  fn decrease_lod(
    &mut self,
    timers: &TimerSet,
    gl: &mut GLContext,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: &BlockPosition,
    requested_lod: LOD,
    owner: OwnerId,
  ) {
    let loaded_lod;
    let new_lod;
    match self.loaded.entry(block_position) {
      Entry::Vacant(_) => return,
      Entry::Occupied(mut entry) => {
        let block_load_state = entry.get_mut();

        match block_load_state.owner_lods.entry(&owner) {
          Entry::Occupied(entry) => {
            let &current_lod = entry.get();
            if current_lod <= requested_lod {
              return;
            }
          },
          Entry::Vacant(_) => return,
        }

        block_load_state.owner_lods.insert(owner, requested_lod);
        new_lod = *block_load_state.owner_lods.values().max_by(|x| *x).unwrap();

        if block_load_state.loaded_lod == new_lod {
          // Already loaded at the right LOD.
          return;
        }

        loaded_lod = Some(block_load_state.loaded_lod);
        block_load_state.loaded_lod = new_lod;
      },
    }

    self.re_lod_block(timers, gl, id_allocator, physics, block_position, loaded_lod, Some(new_lod));
  }

  fn unload(
    &mut self,
    timers: &TimerSet,
    gl: &mut GLContext,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: &BlockPosition,
    owner: OwnerId,
  ) {
    let loaded_lod;
    let new_lod;
    match self.loaded.entry(block_position) {
      Entry::Occupied(mut entry) => {
        {
          let block_load_state = entry.get_mut();
          match block_load_state.loaded_lod {
            LOD::Placeholder => return,
            lod => loaded_lod = lod,
          };

          if block_load_state.owner_lods.remove(&owner).is_none() {
            return;
          }

          match block_load_state.owner_lods.values().max_by(|&x| x) {
            None => {
              new_lod = None;
            },
            Some(&lod) => {
              if lod == loaded_lod {
                return;
              }

              new_lod = Some(lod);
            }
          }
        }
        entry.remove();
      },
      Entry::Vacant(_) => {
        return;
      },
    };

    self.re_lod_block(timers, gl, id_allocator, physics, block_position, Some(loaded_lod), new_lod);
  }

  /// Note that we want a specific `TerrainBlock`.
  fn insert_placeholder(
    &mut self,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
    block_position: &BlockPosition,
    owner: OwnerId,
  ) {
    match self.loaded.entry(block_position) {
      Entry::Occupied(mut entry) => {
        let block_load_state = entry.get_mut();
        match block_load_state.owner_lods.entry(&owner) {
          Entry::Vacant(entry) => { entry.insert(LOD::Placeholder); },
          Entry::Occupied(_) => {},
        }
      },
      Entry::Vacant(entry) => {
        let mut owner_lods = HashMap::new();
        owner_lods.insert(owner, LOD::Placeholder);
        entry.insert(BlockLoadState {
          owner_lods: owner_lods,
          loaded_lod: LOD::Placeholder,
        });
        assert!(self.in_progress_terrain.insert(id_allocator, physics, block_position));
      },
    }
  }

  fn remove_placeholder(
    &mut self,
    physics: &mut Physics,
    block_position: &BlockPosition,
    owner: OwnerId,
  ) {
    match self.loaded.entry(block_position) {
      Entry::Occupied(mut entry) => {
        {
          let block_load_state = entry.get_mut();
          let should_unload =
            block_load_state.loaded_lod == LOD::Placeholder
            && block_load_state.owner_lods.remove(&owner).is_some()
            && block_load_state.owner_lods.is_empty();
          if !should_unload {
            return;
          }
        }
        entry.remove();
      },
      Entry::Vacant(_) => {
        return;
      },
    };

    assert!(self.in_progress_terrain.remove(physics, block_position));
  }
}
