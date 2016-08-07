use cgmath;
use collision;
use rand;
use std;
use time;

use common::entity_id;
use common::{fnv_set, fnv_map};
use common::id_allocator;
use common::surroundings_loader;
use common::voxel;

use chunk;
use client;
use lod;
use record_book;
use terrain_mesh;
use view;

#[derive(Debug, Clone)]
pub enum Load {
  Chunk {
    chunk        : chunk::T,
    requested_at : u64,
  },
  Voxels {
    voxels : Vec<(voxel::bounds::T, voxel::T)>,
  },
}

pub type Chunks = fnv_map::T<chunk::position::T, chunk::T>;

pub struct T {
  /// The chunks we have cached from the server.
  chunks            : Chunks,
  /// A record of all the chunks that have been loaded.
  loaded_chunks     : fnv_map::T<chunk::position::T, (terrain_mesh::Ids, lod::T)>,
  max_load_distance : i32,
  queue             : std::collections::VecDeque<Load>,
}

pub fn new(max_load_distance: i32) -> T {
  T {
    loaded_chunks       : fnv_map::new(),
    chunks       : fnv_map::new(),
    max_load_distance   : max_load_distance,
    queue               : std::collections::VecDeque::new(),
  }
}

pub enum LoadResult { Success, VoxelsMissing, AlreadyLoaded }

impl T {
  pub fn load_state(&self, chunk_position: &chunk::position::T) -> Option<lod::T> {
    self.loaded_chunks
      .get(&chunk_position)
      .map(|&(_, lod)| lod)
  }

  pub fn queued_update_count(&self) -> usize {
    self.queue.len()
  }

  pub fn enqueue(&mut self, msg: Load) {
    self.queue.push_back(msg);
  }

  pub fn tick<Rng, UpdateView>(
    &mut self,
    id_allocator    : &std::sync::Mutex<id_allocator::T<entity_id::T>>,
    rng             : &mut Rng,
    update_view     : &mut UpdateView,
    player_position : &cgmath::Point3<f32>,
  ) where
    UpdateView : FnMut(view::update::T),
    Rng        : rand::Rng,
  {
    let start = time::precise_time_ns();
    while let Some(msg) = self.queue.pop_front() {
      match msg {
        Load::Voxels { voxels } => {
          self.load_voxels(
            id_allocator,
            rng,
            update_view,
            player_position,
            voxels,
          );
        },
        Load::Chunk { chunk, requested_at } => {
          let lod = lod::at_chunk(player_position, &chunk.position);
          self.load_chunk(
            id_allocator,
            rng,
            update_view,
            chunk,
            requested_at,
            lod,
          );
        },
      }

      if time::precise_time_ns() - start >= 1_000_000 {
        break
      }
    }
  }

  #[inline(never)]
  fn force_load_chunk<Rng, UpdateView>(
    &mut self,
    id_allocator   : &std::sync::Mutex<id_allocator::T<entity_id::T>>,
    rng            : &mut Rng,
    update_view    : &mut UpdateView,
    chunk_position : &chunk::position::T,
    lod            : lod::T,
  ) where
    UpdateView : FnMut(view::update::T),
    Rng        : rand::Rng,
  {
    debug!("generate {:?} at {:?}", chunk_position, lod);
    let mesh_chunk =
      terrain_mesh::generate(
        &self.chunks,
        &chunk_position,
        lod,
        id_allocator,
        rng,
      );

    let mut updates = Vec::new();

    use std::collections::hash_map::Entry::*;
    // TODO: Rc instead of clone.
    match self.loaded_chunks.entry(*chunk_position) {
      Vacant(entry) => {
        entry.insert((mesh_chunk.ids(), lod));
      },
      Occupied(mut entry) => {
        let (ids, _) = entry.insert((mesh_chunk.ids(), lod));
        updates.push(view::update::UnloadChunk { ids: ids });
      },
    };

    if !mesh_chunk.ids.is_empty() {
      updates.push(
        view::update::LoadChunk {
          mesh     : mesh_chunk,
        }
      );
    }

    update_view(view::update::Atomic(updates));
  }

  /// Generate view updates to load as much of a chunk as is possible.
  pub fn try_load_chunk<Rng, UpdateView>(
    &mut self,
    id_allocator   : &std::sync::Mutex<id_allocator::T<entity_id::T>>,
    rng            : &mut Rng,
    update_view    : &mut UpdateView,
    chunk_position : &chunk::position::T,
  ) -> LoadResult where
    UpdateView : FnMut(view::update::T),
    Rng        : rand::Rng,
  {
    let mut r = LoadResult::Success;
    let already_loaded = self.loaded_chunks.contains_key(chunk_position);
    if already_loaded {
      debug!("Not re-loading {:?}", chunk_position);
      return LoadResult::AlreadyLoaded;
    }

    if self.chunks.contains_key(chunk_position);
    match self.load_chunk(update_view, chunk_position) {
      Ok(())  => {},
      Err(()) => r = LoadResult::VoxelsMissing,
    }

    r
  }

  #[inline(never)]
  fn load_voxel<UpdateChunk>(
    &mut self,
    player_position  : &cgmath::Point3<f32>,
    voxel            : voxel::T,
    bounds           : &voxel::bounds::T,
    mut update_chunk : UpdateChunk,
  ) where
    UpdateChunk: FnMut(chunk::position::T, lod::T),
  {
    unimplemented!();
  }

  #[inline(never)]
  fn load_voxels<Rng, UpdateView>(
    &mut self,
    id_allocator    : &std::sync::Mutex<id_allocator::T<entity_id::T>>,
    rng             : &mut Rng,
    update_view     : &mut UpdateView,
    player_position : &cgmath::Point3<f32>,
    voxel_updates   : Vec<(voxel::bounds::T, voxel::T)>,
  ) where
    UpdateView : FnMut(view::update::T),
    Rng        : rand::Rng,
  {
    unimplemented!();
  }

  #[inline(never)]
  fn load_chunk<Rng, UpdateView>(
    &mut self,
    id_allocator    : &std::sync::Mutex<id_allocator::T<entity_id::T>>,
    rng             : &mut Rng,
    update_view     : &mut UpdateView,
    chunk           : chunk::T,
    requested_at    : u64,
    lod             : lod::T,
  ) where
    UpdateView : FnMut(view::update::T),
    Rng        : rand::Rng,
  {
    let mut update_chunks = fnv_set::new();
    let response_time = time::precise_time_ns();
    self.chunks.insert(chunk.position, chunk);

    let processed_time = time::precise_time_ns();
    for (chunk, lod) in update_chunks.into_iter() {
      let _ =
        self.force_load_chunk(
          id_allocator,
          rng,
          update_view,
          &chunk,
          lod,
        );
    }

    let chunk_loaded = time::precise_time_ns();

    record_book::thread_local::push_chunk_load(
      record_book::ChunkLoad {
        requested_at : requested_at,
        responded_at : response_time,
        processed_at : processed_time,
        loaded_at    : chunk_loaded,
      },
    );
  }

  pub fn unload<UpdateView>(
    &mut self,
    update_view    : &mut UpdateView,
    chunk_position : &chunk::position::T,
  ) where
    UpdateView : FnMut(view::update::T),
  {
    match self.loaded_chunks.remove(chunk_position) {
      None => {},
      Some((ids, _)) => {
        update_view(view::update::UnloadChunk { ids: ids });
      },
    }
  }
}
