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
    chunk         : chunk::T,
    position      : chunk::position::T,
    lg_voxel_size : i16,
    requested_at  : u64,
  },
  Voxels {
    voxels : Vec<(voxel::bounds::T, voxel::T)>,
  },
}

pub type Chunks = fnv_map::T<(chunk::position::T, i16), chunk::T>;

struct LoadState {
  mesh_ids : terrain_mesh::Ids,
  lod      : lod::T,
}

pub struct T {
  /// The chunks we have cached from the server.
  chunks            : Chunks,
  /// A record of all the chunks that have been loaded.
  loaded_chunks     : fnv_map::T<chunk::position::T, LoadState>,
  max_load_distance : i32,
  queue             : std::collections::VecDeque<Load>,
}

pub fn new(max_load_distance: i32) -> T {
  T {
    loaded_chunks     : fnv_map::new(),
    chunks            : fnv_map::new(),
    max_load_distance : max_load_distance,
    queue             : std::collections::VecDeque::new(),
  }
}

pub enum LoadResult { Success, ChunkMissing, AlreadyLoaded }

impl T {
  pub fn load_state(&self, chunk_position: &chunk::position::T) -> Option<lod::T> {
    self.loaded_chunks
      .get(&chunk_position)
      .map(|&load_state| load_state.lod)
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
        Load::Chunk { chunk, requested_at, position, lg_voxel_size, .. } => {
          let lod =
            lod::of_distance(
              surroundings_loader::distance_between(
                &chunk::position::of_world_position(player_position).as_point,
                &position.as_point,
              )
            );
          self.load_chunk(
            id_allocator,
            rng,
            update_view,
            position,
            chunk,
            requested_at,
            lg_voxel_size,
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
        entry.insert(
          LoadState {
            mesh_ids : mesh_chunk.ids(),
            lod      : lod,
          }
        );
      },
      Occupied(mut entry) => {
        let load_state =
          entry.insert(
            LoadState {
              mesh_ids : mesh_chunk.ids(),
              lod      : lod,
            }
          );
        updates.push(view::update::UnloadChunk { ids: load_state.mesh_ids });
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
    lod            : lod::T,
  ) -> LoadResult where
    UpdateView : FnMut(view::update::T),
    Rng        : rand::Rng,
  {
    let mut r = LoadResult::Success;
    let already_loaded =
      match self.loaded_chunks.get(chunk_position) {
        None             => false,
        Some(load_state) => load_state.lod == lod,
      };
    if already_loaded {
      debug!("Not re-loading {:?}", (chunk_position, lod));
      return LoadResult::AlreadyLoaded
    }

    let lg_voxel_size = terrain_mesh::LG_SAMPLE_SIZE[lod.0 as usize];
    if !self.chunks.contains_key(&(*chunk_position, lg_voxel_size)) {
      return LoadResult::ChunkMissing
    }

    self.force_load_chunk(id_allocator, rng, update_view, chunk_position, lod);

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
    id_allocator  : &std::sync::Mutex<id_allocator::T<entity_id::T>>,
    rng           : &mut Rng,
    update_view   : &mut UpdateView,
    position      : chunk::position::T,
    chunk         : chunk::T,
    requested_at  : u64,
    lg_voxel_size : i16,
  ) where
    UpdateView : FnMut(view::update::T),
    Rng        : rand::Rng,
  {
    let response_time = time::precise_time_ns();
    self.chunks.insert((position, lg_voxel_size), chunk);
    let processed_time = time::precise_time_ns();

    for i in 0 .. terrain_mesh::LOD_COUNT {
      if terrain_mesh::LG_SAMPLE_SIZE[i] == lg_voxel_size {
        let lod = lod::T(i as u32);
        self.force_load_chunk(
          id_allocator,
          rng,
          update_view,
          &position,
          lod,
        );
      }
    }

    let chunk_loaded = time::precise_time_ns();

    record_book::thread_local::push_chunk_load(
      record_book::ChunkLoad {
        requested     : requested_at,
        responded     : response_time,
        voxels_loaded : processed_time,
        edges_loaded  : chunk_loaded,
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
      Some(load_state) => {
        update_view(view::update::UnloadChunk { ids: load_state.mesh_ids });
      },
    }
  }
}
