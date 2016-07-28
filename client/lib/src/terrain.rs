use cgmath;
use collision;
use rand;
use std;
use time;

use common::entity_id;
use common::id_allocator;
use common::surroundings_loader;
use common::voxel;

use chunk_position;
use client;
use lod;
use record_book;
use terrain_mesh;
use view;

#[derive(Debug, Clone)]
pub enum Load {
  Voxels {
    voxels       : Vec<(voxel::bounds::T, voxel::T)>,
    request_time : Option<u64>,
  },
}

pub struct T {
  /// A record of all the chunks that have been loaded.
  loaded_chunks       : chunk_position::map::T<(terrain_mesh::T, lod::T)>,
  /// Map each chunk to the number of voxels inside it that we have.
  chunk_voxels_loaded : chunk_position::with_lod::map::T<u32>,
  /// The voxels we have cached from the server.
  voxels              : voxel::tree::T,
  max_load_distance   : i32,
  queue               : std::collections::VecDeque<Load>,
}

pub fn new(max_load_distance: i32) -> T {
  T {
    loaded_chunks       : chunk_position::map::new(),
    chunk_voxels_loaded : chunk_position::with_lod::map::new(),
    voxels              : voxel::tree::new(),
    max_load_distance   : max_load_distance,
    queue               : std::collections::VecDeque::new(),
  }
}

impl T {
  pub fn load_state(&self, chunk_position: &chunk_position::T) -> Option<lod::T> {
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
    chunk_position: chunk_position::T,
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
        Load::Voxels { voxels, request_time } => {
          self.load_voxels(
            id_allocator,
            rng,
            update_view,
            player_position,
            voxels,
            request_time,
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
    chunk_position : &chunk_position::T,
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
        entry.insert((mesh_chunk.clone(), lod));
      },
      Occupied(mut entry) => {
        {
          // The mesh_chunk removal code is duplicated in update_thread.

          let &(ref prev_chunk, _) = entry.get();
          for id in &prev_chunk.grass_ids {
            updates.push(view::update::RemoveGrass(*id));
          }
          for &id in &prev_chunk.ids {
            updates.push(view::update::RemoveTerrain(id));
          }
        }
        entry.insert((mesh_chunk.clone(), lod));
      },
    };

    if !mesh_chunk.ids.is_empty() {
      updates.push(view::update::AddChunk(*chunk_position, mesh_chunk, lod));
    }

    update_view(view::update::Atomic(updates));
  }

  pub fn load_chunk<Rng, UpdateView>(
    &mut self,
    id_allocator   : &std::sync::Mutex<id_allocator::T<entity_id::T>>,
    rng            : &mut Rng,
    update_view    : &mut UpdateView,
    chunk_position : &chunk_position::T,
    lod            : lod::T,
  ) -> Result<(), Vec<voxel::bounds::T>> where
    UpdateView : FnMut(view::update::T),
    Rng        : rand::Rng,
  {
    let all_voxels_loaded =
      self.all_voxels_loaded(
        *chunk_position,
        lod,
      );
    if all_voxels_loaded {
      self.force_load_chunk(
        id_allocator,
        rng,
        update_view,
        chunk_position,
        lod,
      );
      Ok(())
    } else {
      let voxel_size = 1 << terrain_mesh::LG_SAMPLE_SIZE[lod.0 as usize];
      let voxels =
        terrain_mesh::voxels_in(
          &collision::Aabb3::new(
            cgmath::Point3::new(
              (chunk_position.as_pnt().x << terrain_mesh::LG_WIDTH) - voxel_size,
              (chunk_position.as_pnt().y << terrain_mesh::LG_WIDTH) - voxel_size,
              (chunk_position.as_pnt().z << terrain_mesh::LG_WIDTH) - voxel_size,
            ),
            cgmath::Point3::new(
              ((chunk_position.as_pnt().x + 1) << terrain_mesh::LG_WIDTH) + voxel_size,
              ((chunk_position.as_pnt().y + 1) << terrain_mesh::LG_WIDTH) + voxel_size,
              ((chunk_position.as_pnt().z + 1) << terrain_mesh::LG_WIDTH) + voxel_size,
            ),
          ),
          terrain_mesh::LG_SAMPLE_SIZE[lod.0 as usize],
        );
      Err(voxels)
    }
  }

  #[inline(never)]
  fn load_voxel<UpdateChunk>(
    &mut self,
    player_position  : &cgmath::Point3<f32>,
    voxel            : voxel::T,
    bounds           : &voxel::bounds::T,
    mut update_chunk : UpdateChunk,
  ) where
    UpdateChunk: FnMut(chunk_position::T, lod::T),
  {
    let player_position = chunk_position::of_world_position(player_position);

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
    let mut updated_lod = None;
    for lod in 0..terrain_mesh::LOD_COUNT as u32 {
      let lod = lod::T(lod);

      let lg_size = terrain_mesh::LG_SAMPLE_SIZE[lod.0 as usize];
      if lg_size == bounds.lg_size {
        updated_lod = Some(lod);
        break
      }
    }

    for chunk_position in updated_chunk_positions(&bounds).into_iter() {
      trace!("chunk_position {:?}", chunk_position);
      if new_voxel_loaded {
        match updated_lod {
          None => {}
          Some(updated_lod) => {
            let chunk_voxels_loaded =
              self.chunk_voxels_loaded.entry((chunk_position, updated_lod))
              .or_insert_with(|| 0);
            trace!("{:?} gets {:?}", chunk_position, bounds);
            *chunk_voxels_loaded += 1;
          },
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
  fn load_voxels<Rng, UpdateView>(
    &mut self,
    id_allocator    : &std::sync::Mutex<id_allocator::T<entity_id::T>>,
    rng             : &mut Rng,
    update_view     : &mut UpdateView,
    player_position : &cgmath::Point3<f32>,
    voxel_updates   : Vec<(voxel::bounds::T, voxel::T)>,
    request_time    : Option<u64>,
  ) where
    UpdateView : FnMut(view::update::T),
    Rng        : rand::Rng,
  {
    let mut update_chunks = chunk_position::with_lod::set::new();
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
    chunk_position : &chunk_position::T,
  ) where
    UpdateView : FnMut(view::update::T),
  {
    self.loaded_chunks
      .remove(chunk_position)
      // If it wasn't loaded, don't unload anything.
      .map(|(chunk, _)| {
        for id in &chunk.grass_ids {
          update_view(view::update::RemoveGrass(*id));
        }
        for id in &chunk.ids {
          update_view(view::update::RemoveTerrain(*id));
        }
      });
  }
}

#[inline(never)]
fn updated_chunk_positions(
  voxel: &voxel::bounds::T,
) -> Vec<chunk_position::T>
{
  let chunk = chunk_position::containing_voxel(voxel);

  macro_rules! tweak(($dim:ident) => {{
    let mut new_voxel = voxel.clone();
    new_voxel.$dim += 1;
    if chunk_position::containing_voxel(&new_voxel) == chunk {
      let mut new_voxel = voxel.clone();
      new_voxel.$dim -= 1;
      if chunk_position::containing_voxel(&new_voxel) == chunk {
        0
      } else {
        -1
      }
    } else {
      1
    }
  }});

  let tweak =
    cgmath::Point3::new(
      tweak!(x),
      tweak!(y),
      tweak!(z),
    );

  macro_rules! consider(($dim:ident, $chunk:expr, $next:expr) => {{
    $next($chunk);
    if tweak.$dim != 0 {
      let mut chunk = $chunk;
      chunk.as_mut_pnt().$dim += tweak.$dim;
      $next(chunk);
    }
  }});

  let mut chunks = Vec::new();
  consider!(x, chunk, |chunk: chunk_position::T| {
  consider!(y, chunk, |chunk: chunk_position::T| {
  consider!(z, chunk, |chunk: chunk_position::T| {
    chunks.push(chunk);
  })})});

  chunks
}
