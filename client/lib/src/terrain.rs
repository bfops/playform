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

pub struct T {
  /// A record of all the chunks that have been loaded.
  loaded_chunks       : fnv_map::T<chunk::position::T, (terrain_mesh::Ids, lod::T)>,
  /// The voxels we have cached from the server.
  voxels              : voxel::tree::T,
  max_load_distance   : i32,
  queue               : std::collections::VecDeque<Load>,
}

pub fn new(max_load_distance: i32) -> T {
  T {
    loaded_chunks       : fnv_map::new(),
    voxels              : voxel::tree::new(),
    max_load_distance   : max_load_distance,
    queue               : std::collections::VecDeque::new(),
  }
}

pub enum LoadResult { Success, VoxelsMissing }

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

  fn all_voxels_loaded(
    &self,
    chunk_position: chunk::position::T,
    lod: lod::T,
  ) -> bool {
    let chunk_voxels_loaded =
      match self.chunk_voxels_loaded.get(&(chunk_position, lod)) {
        None => return false,
        Some(x) => x,
      };

    let edge_samples = terrain_mesh::EDGE_SAMPLES[lod.0 as usize] as u32 + 2;
    let samples = edge_samples * edge_samples * edge_samples;
    assert!(*chunk_voxels_loaded <= samples, "{:?}", chunk_position);
    *chunk_voxels_loaded == samples
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
        Load::Voxels { voxels, requested_at } => {
          self.load_chunks(
            id_allocator,
            rng,
            update_view,
            player_position,
            voxels,
            requested_at,
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
    let mesh_chunk = terrain_mesh::generate(&self.voxels, &chunk_position, lod, id_allocator, rng);

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
    for edge in chunk::edges(chunk_position) {
      let already_loaded = self.loaded_edges.lock().unwrap().contains_key(&edge);
      if already_loaded {
        debug!("Not re-loading {:?}", chunk_position);
        continue;
      }

      match self.load_edge(update_view, edge) {
        Ok(()) => {},
        Err(()) => r = LoadResult::VoxelsMissing,
      }
    }
  }

  pub fn load_chunk<Rng, UpdateView>(
    &mut self,
    id_allocator   : &std::sync::Mutex<id_allocator::T<entity_id::T>>,
    rng            : &mut Rng,
    update_view    : &mut UpdateView,
    chunk          : &chunk::T,
  ) where
    UpdateView : FnMut(view::update::T),
    Rng        : rand::Rng,
  {
    self.force_load_chunk(
      id_allocator,
      rng,
      update_view,
      chunk_position,
      lod,
    );
    Ok(())
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
    let player_position = chunk::position::T::of_world_position(player_position);

    // Has a new voxel been loaded? (or did we change an existing voxel)
    let new_voxel_loaded;
    {
      let voxel = Some(voxel);
      let node = self.voxels.get_mut_or_create(bounds);
      let old_voxel = &mut node.data;
      new_voxel_loaded = old_voxel.is_none();
      if *old_voxel == voxel {
        return
      }
      *old_voxel = voxel;
    }

    trace!("voxel bounds {:?}", bounds);

    // The LOD of the chunks that should be updated.
    // This doesn't necessarily match the LOD they're loaded at.
    let mut updated_lods = Vec::new();
    for lod in 0..terrain_mesh::LOD_COUNT as u32 {
      let lod = lod::T(lod);

      let lg_size = terrain_mesh::LG_SAMPLE_SIZE[lod.0 as usize];
      if lg_size == bounds.lg_size {
        updated_lods.push(lod);
      }
    }

    for chunk_position in updated_chunk_positions(&bounds).into_iter() {
      trace!("chunk_position {:?}", chunk_position);
      if new_voxel_loaded {
        for &updated_lod in &updated_lods {
          let chunk_voxels_loaded =
            self.chunk_voxels_loaded.entry((chunk_position, updated_lod))
            .or_insert_with(|| 0);
          trace!("{:?} gets {:?}", chunk_position, bounds);
          *chunk_voxels_loaded += 1;
        }
      }

      let distance =
        surroundings_loader::distance_between(
          player_position.as_pnt(),
          &chunk_position.as_pnt(),
        );

      if distance > self.max_load_distance {
        debug!(
          "Not loading {:?}: too far away from player at {:?}.",
          bounds,
          player_position,
        );
        continue;
      }

      let lod = client::lod_index(distance);
      let lg_size = terrain_mesh::LG_SAMPLE_SIZE[lod.0 as usize];
      if lg_size != bounds.lg_size {
        debug!(
          "{:?} is not the desired LOD {:?}.",
          bounds,
          lod
        );
        continue;
      }

      if self.all_voxels_loaded(chunk_position, lod) {
        update_chunk(chunk_position, lod);
      }
    }
  }

  #[inline(never)]
  fn load_edge<Rng, UpdateView>(
    &mut self,
    id_allocator : &std::sync::Mutex<id_allocator::T<entity_id::T>>,
    rng          : &mut Rng,
    update_view  : &mut UpdateView,
    edge         : &edge::T,
  ) -> Result<(), ()> where
    UpdateView : FnMut(view::update::T),
    Rng        : rand::Rng,
  {
    debug!("generate {:?}", edge);
    let mesh_fragment = try!(terrain_mesh::generate(&self.voxels, edge, &id_allocator, rng));

    let mut updates = Vec::new();

    let unload_fragments = self.loaded_edges.insert(&edge, mesh_fragment.clone());

    for mesh_fragment in unload_fragments {
      for id in &mesh_fragment.ids {
        updates.push(view::update::RemoveTerrain(*id));
      }
      for id in &mesh_fragment.grass_ids {
        updates.push(view::update::RemoveGrass(*id));
      }
    }

    if !mesh_fragment.ids.is_empty() {
      updates.push(view::update::LoadMesh(mesh_fragment));
    }

    update_view(view::update::Atomic(updates));

    debug!("generate success!");
    Ok(())
  }

  #[inline(never)]
  fn load_chunk<Rng, UpdateView>(
    &mut self,
    id_allocator    : &std::sync::Mutex<id_allocator::T<entity_id::T>>,
    rng             : &mut Rng,
    update_view     : &mut UpdateView,
    player_position : &cgmath::Point3<f32>,
    voxel_updates   : Vec<(voxel::bounds::T, voxel::T)>,
    requested_at    : Option<u64>,
  ) where
    UpdateView : FnMut(view::update::T),
    Rng        : rand::Rng,
  {
    let mut update_chunks = fnv_set::new();
    let response_time = time::precise_time_ns();
    for (bounds, voxel) in voxel_updates {
      trace!("Got voxel at {:?}", bounds);
      self.load_voxel(
        player_position,
        voxel,
        &bounds,
        |chunk, lod| { update_chunks.insert((chunk, lod)); },
      );
    }

    let processed_time = time::precise_time_ns();
    for (chunk, lod) in update_chunks.into_iter() {
      let _ =
        self.load_chunk(
          id_allocator,
          rng,
          update_view,
          &chunk,
          lod,
        );
    }

    let chunk_loaded = time::precise_time_ns();

    match request_time {
      None => {},
      Some(request_time) => {
        record_book::thread_local::push_chunk_load(
          record_book::ChunkLoad {
            requested_at : request_time,
            responded_at : response_time,
            processed_at : processed_time,
            loaded_at    : chunk_loaded,
          }
        );
      },
    }
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
