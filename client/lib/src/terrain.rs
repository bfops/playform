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
  loaded_chunks       : fnv_map::T<chunk::position::T, (terrain_mesh::Ids, lod::T)>,
  /// Map each chunk to the number of voxels inside it that we have.
  chunk_voxels_loaded : fnv_map::T<(chunk::position::T, lod::T), u32>,
  /// The voxels we have cached from the server.
  voxels              : voxel::tree::T,
  max_load_distance   : i32,
  queue               : std::collections::VecDeque<Load>,
}

pub fn new(max_load_distance: i32) -> T {
  T {
    loaded_chunks       : fnv_map::new(),
    chunk_voxels_loaded : fnv_map::new(),
    voxels              : voxel::tree::new(),
    max_load_distance   : max_load_distance,
    queue               : std::collections::VecDeque::new(),
  }
}

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

    let edge_samples = lod.edge_samples() + 2;
    let samples = edge_samples * edge_samples * edge_samples;
    assert!(*chunk_voxels_loaded <= samples as u32, "{:?}", chunk_position);
    *chunk_voxels_loaded == samples as u32
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

  pub fn load_chunk<Rng, UpdateView>(
    &mut self,
    id_allocator   : &std::sync::Mutex<id_allocator::T<entity_id::T>>,
    rng            : &mut Rng,
    update_view    : &mut UpdateView,
    chunk_position : &chunk::position::T,
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
      let voxel_size = 1 << lod.lg_sample_size();
      let voxels =
        terrain_mesh::voxels_in(
          &collision::Aabb3::new(
            cgmath::Point3::new(
              (chunk_position.as_pnt().x << chunk::LG_WIDTH) - voxel_size,
              (chunk_position.as_pnt().y << chunk::LG_WIDTH) - voxel_size,
              (chunk_position.as_pnt().z << chunk::LG_WIDTH) - voxel_size,
            ),
            cgmath::Point3::new(
              ((chunk_position.as_pnt().x + 1) << chunk::LG_WIDTH) + voxel_size,
              ((chunk_position.as_pnt().y + 1) << chunk::LG_WIDTH) + voxel_size,
              ((chunk_position.as_pnt().z + 1) << chunk::LG_WIDTH) + voxel_size,
            ),
          ),
          lod.lg_sample_size(),
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
    UpdateChunk: FnMut(chunk::position::T, lod::T),
  {
    let player_position = chunk::position::of_world_position(player_position);

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
    for lod in 0..lod::COUNT as u32 {
      let lod = lod::T(lod);

      let lg_size = lod.lg_sample_size();
      if lg_size == bounds.lg_size {
        updated_lods.push(lod);
      }
    }

    for chunk_position in updated_chunk_positions(&bounds) {
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

      let lod = lod::of_distance(distance as u32);
      let lg_size = lod.lg_sample_size();
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
    for (chunk, lod) in update_chunks {
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
            request_time_ns  : request_time,
            response_time_ns : response_time,
            stored_time_ns   : processed_time,
            loaded_time_ns   : chunk_loaded,
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

#[inline(never)]
fn updated_chunk_positions(
  voxel: &voxel::bounds::T,
) -> Vec<chunk::position::T>
{
  let chunk = chunk::position::containing_voxel(voxel);

  macro_rules! tweak(($dim:ident) => {{
    let mut new_voxel = voxel.clone();
    new_voxel.$dim += 1;
    if chunk::position::containing_voxel(&new_voxel) == chunk {
      let mut new_voxel = voxel.clone();
      new_voxel.$dim -= 1;
      if chunk::position::containing_voxel(&new_voxel) == chunk {
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
  consider!(x, chunk, |chunk: chunk::position::T| {
  consider!(y, chunk, |chunk: chunk::position::T| {
  consider!(z, chunk, |chunk: chunk::position::T| {
    chunks.push(chunk);
  })})});

  chunks
}
