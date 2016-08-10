use rand;
use std;
use time;

use common::entity_id;
use common::fnv_map;
use common::id_allocator;
use common::surroundings_loader;
use common::voxel;

use chunk;
use edge;
use lod;
use record_book;
use terrain_mesh;
use view;

#[derive(Debug, Clone)]
pub enum Load {
  Chunk {
    chunk           : chunk::T,
    position        : chunk::position::T,
    lg_voxel_size   : i16,
    request_time_ns : u64,
  },
  Voxels {
    voxels : Vec<(voxel::bounds::T, voxel::T)>,
  },
}

pub type Chunks = fnv_map::T<(chunk::position::T, i16), chunk::T>;

struct LoadState {
  lod : lod::T,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, RustcEncodable, RustcDecodable)]
enum MeshId {
  ChunkInner(chunk::position::T),
  // x,y,z faces in the negative direction from a chunk position.
  ChunkFace(chunk::position::T, edge::Direction),
}

pub struct T {
  /// The chunks we have cached from the server.
  chunks            : Chunks,
  /// A record of all the chunks that have been loaded.
  chunk_load_state  : fnv_map::T<chunk::position::T, LoadState>,
  /// This maps mesh ids (one per chunk and one per inter-chunk seam) to their
  /// VRAM-lookup entity IDs.
  loaded_meshes     : fnv_map::T<MeshId, terrain_mesh::Ids>,
  max_load_distance : i32,
  queue             : std::collections::VecDeque<Load>,
}

pub fn new(max_load_distance: i32) -> T {
  T {
    chunks            : fnv_map::new(),
    chunk_load_state  : fnv_map::new(),
    loaded_meshes     : fnv_map::new(),
    max_load_distance : max_load_distance,
    queue             : std::collections::VecDeque::new(),
  }
}

pub enum LoadResult { Success, ChunkMissing, AlreadyLoaded }

impl T {
  pub fn load_state(&self, chunk_position: &chunk::position::T) -> Option<lod::T> {
    self.chunk_load_state
      .get(&chunk_position)
      .map(|load_state| load_state.lod)
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
    player_position : &chunk::position::T,
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
        Load::Chunk { chunk, request_time_ns, position, lg_voxel_size, .. } => {
          self.load_chunk(
            id_allocator,
            rng,
            update_view,
            player_position,
            position,
            chunk,
            request_time_ns,
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
    id_allocator    : &std::sync::Mutex<id_allocator::T<entity_id::T>>,
    rng             : &mut Rng,
    update_view     : &mut UpdateView,
    player_position : &chunk::position::T,
    chunk_position  : &chunk::position::T,
    lod             : lod::T,
  ) where
    UpdateView : FnMut(view::update::T),
    Rng        : rand::Rng,
  {
    debug!("generate {:?} at {:?}", chunk_position, lod);
    let mut updates = Vec::new();

    self.chunk_load_state.insert(
      *chunk_position,
      LoadState {
        lod: lod,
      },
    );

    {
      let loaded_meshes = &mut self.loaded_meshes;
      let mut load_mesh = |id, mesh: terrain_mesh::T| {
        use std::collections::hash_map::Entry::*;
        match loaded_meshes.entry(id) {
          Vacant(entry) => {
            entry.insert(mesh.ids());
          },
          Occupied(mut entry) => {
            let previous_ids = entry.insert(mesh.ids());
            updates.push(view::update::UnloadMesh { ids: previous_ids });
          },
        };
        updates.push(
          view::update::LoadMesh {
            mesh : mesh,
          }
        );
      };

      let lg_size = lod::LG_SAMPLE_SIZE[lod.0 as usize];

      load_mesh(
        MeshId::ChunkInner(*chunk_position),
        terrain_mesh::of_edges(
          &self.chunks,
          *player_position,
          lod,
          id_allocator,
          rng,
          chunk::position::inner_edges(*chunk_position, lg_size),
        ),
      );

      let chunks = &self.chunks;
      let max_load_distance = self.max_load_distance;
      let desired_lod = |chunk_positions: &[chunk::position::T]| {
        let mut lod = lod;
        for p in chunk_positions {
          let distance =
            surroundings_loader::distance_between(
              &p.as_point,
              &player_position.as_point,
            );

          if distance > max_load_distance {
            return None;
          }

          let this_lod = lod::of_distance(distance);

          if !chunks.contains_key(&(*p, this_lod.lg_sample_size())) {
            return None
          }

          // TODO: Remove this conditional block. It disables LOD seam fixing.
          if lod != this_lod {
            return None
          }

          lod = std::cmp::max(lod, this_lod);
        }
        Some(lod)
      };

      let mut load_face = |d, chunk_position| {
        let id = MeshId::ChunkFace(chunk_position, d);

        let lod =
          match desired_lod(&[chunk_position, chunk_position - d.to_vec()]) {
            None => return,
            Some(lod) => lod,
          };
        let lg_size = lod.lg_sample_size();

        load_mesh(
          id,
          terrain_mesh::of_edges(
            chunks,
            *player_position,
            lod,
            id_allocator,
            rng,
            chunk::position::face_edges(d, chunk_position, lg_size),
          ),
        );
      };

      load_face(edge::Direction::X, *chunk_position);
      load_face(edge::Direction::Y, *chunk_position);
      load_face(edge::Direction::Z, *chunk_position);

      let mut load_face = |d: edge::Direction, chunk_position: chunk::position::T| {
        // NOT recursive. calls load_face from before.
        load_face(d, chunk_position + d.to_vec());
      };

      load_face(edge::Direction::X, *chunk_position);
      load_face(edge::Direction::Y, *chunk_position);
      load_face(edge::Direction::Z, *chunk_position);
    }

    update_view(view::update::Atomic(updates));
  }

  /// Generate view updates to load as much of a chunk as is possible.
  pub fn try_load_chunk<Rng, UpdateView>(
    &mut self,
    id_allocator    : &std::sync::Mutex<id_allocator::T<entity_id::T>>,
    rng             : &mut Rng,
    update_view     : &mut UpdateView,
    player_position : &chunk::position::T,
    chunk_position  : &chunk::position::T,
    lod             : lod::T,
  ) -> LoadResult where
    UpdateView : FnMut(view::update::T),
    Rng        : rand::Rng,
  {
    let already_loaded =
      match self.chunk_load_state.get(chunk_position) {
        None             => false,
        Some(load_state) => load_state.lod == lod,
      };
    if already_loaded {
      debug!("Not re-loading {:?}", (chunk_position, lod));
      return LoadResult::AlreadyLoaded
    }

    let lg_voxel_size = lod::LG_SAMPLE_SIZE[lod.0 as usize];
    if !self.chunks.contains_key(&(*chunk_position, lg_voxel_size)) {
      return LoadResult::ChunkMissing
    }

    self.force_load_chunk(id_allocator, rng, update_view, player_position, chunk_position, lod);
    LoadResult::Success
  }

  #[inline(never)]
  fn load_voxels<Rng, UpdateView>(
    &mut self,
    _id_allocator    : &std::sync::Mutex<id_allocator::T<entity_id::T>>,
    _rng             : &mut Rng,
    _update_view     : &mut UpdateView,
    _player_position : &chunk::position::T,
    _voxel_updates   : Vec<(voxel::bounds::T, voxel::T)>,
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
    player_position : &chunk::position::T,
    position        : chunk::position::T,
    chunk           : chunk::T,
    request_time_ns : u64,
    lg_voxel_size   : i16,
  ) where
    UpdateView : FnMut(view::update::T),
    Rng        : rand::Rng,
  {
    let response_time = time::precise_time_ns();
    self.chunks.insert((position, lg_voxel_size), chunk);
    let processed_time = time::precise_time_ns();

    let distance =
      surroundings_loader::distance_between(
        &position.as_point,
        &player_position.as_point,
      );
    if distance > self.max_load_distance {
      return
    }

    let lod = lod::of_distance(distance);

    if lod::LG_SAMPLE_SIZE[lod.0 as usize] != lg_voxel_size {
      return
    }

    self.force_load_chunk(
      id_allocator,
      rng,
      update_view,
      player_position,
      &position,
      lod,
    );

    let chunk_loaded = time::precise_time_ns();

    record_book::thread_local::push_chunk_load(
      record_book::ChunkLoad {
        request_time_ns  : request_time_ns,
        response_time_ns : response_time,
        stored_time_ns   : processed_time,
        loaded_time_ns   : chunk_loaded,
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
    let removed = self.chunk_load_state.remove(chunk_position);
    if removed.is_none() {
      return
    }

    let mut remove_mesh = |id| {
      match self.loaded_meshes.remove(&id) {
        None => {},
        Some(mesh) => {
          update_view(view::update::UnloadMesh { ids: mesh });
        },
      }
    };

    remove_mesh(MeshId::ChunkInner(*chunk_position));
    remove_mesh(MeshId::ChunkFace(*chunk_position, edge::Direction::X));
    remove_mesh(MeshId::ChunkFace(*chunk_position, edge::Direction::Y));
    remove_mesh(MeshId::ChunkFace(*chunk_position, edge::Direction::Z));
    remove_mesh(MeshId::ChunkFace(*chunk_position + edge::Direction::X.to_vec(), edge::Direction::X));
    remove_mesh(MeshId::ChunkFace(*chunk_position + edge::Direction::Y.to_vec(), edge::Direction::Y));
    remove_mesh(MeshId::ChunkFace(*chunk_position + edge::Direction::Z.to_vec(), edge::Direction::Z));
  }
}
