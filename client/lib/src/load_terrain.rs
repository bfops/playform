use cgmath::Point3;
use num;

use common::surroundings_loader;
use common::voxel;

use chunk_position;
use client;
use lod;
use terrain_mesh;
use view;

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
    Point3::new(
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

pub fn all_voxels_loaded(
  chunk_voxels_loaded: &chunk_position::with_lod::map::T<u32>,
  chunk_position: chunk_position::T,
  lod: lod::T,
) -> bool {
  let chunk_voxels_loaded =
    match chunk_voxels_loaded.get(&(chunk_position, lod)) {
      None => return false,
      Some(x) => x,
    };

  let edge_samples = terrain_mesh::EDGE_SAMPLES[lod.0 as usize] as u32 + 2;
  let samples = edge_samples * edge_samples * edge_samples;
  assert!(*chunk_voxels_loaded <= samples, "{:?}", chunk_position);
  *chunk_voxels_loaded == samples
}

#[inline(never)]
pub fn load_voxel<UpdateChunk>(
  client: &client::T,
  voxel: voxel::T,
  bounds: &voxel::bounds::T,
  mut update_chunk: UpdateChunk,
) where
  UpdateChunk: FnMut(chunk_position::T, lod::T),
{
  let player_position =
    chunk_position::of_world_position(&client.player_position.lock().unwrap());

  let mut voxels = client.voxels.lock().unwrap();
  let mut chunk_voxels_loaded = client.chunk_voxels_loaded.lock().unwrap();

  // Has a new voxel been loaded? (in contrast to changing an existing voxel)
  let new_voxel_loaded;
  {
    let voxel = Some(voxel);
    let node = voxels.get_mut_or_create(bounds);
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
            chunk_voxels_loaded.entry((chunk_position, updated_lod))
            .or_insert_with(|| 0);
          trace!("{:?} gets {:?}", chunk_position, bounds);
          *chunk_voxels_loaded += 1;
        },
      }
    }

    let distance = surroundings_loader::distance_between(player_position.as_pnt(), &chunk_position.as_pnt());

    if distance > client.max_load_distance {
      debug!(
        "Not loading {:?}: too far away from player at {:?}.",
        bounds,
        player_position,
      );
      continue;
    }

    let lod = lod_index(distance);
    let lg_size = terrain_mesh::LG_SAMPLE_SIZE[lod.0 as usize];
    if lg_size != bounds.lg_size {
      debug!(
        "{:?} is not the desired LOD {:?}.",
        bounds,
        lod
      );
      continue;
    }

    if all_voxels_loaded(&chunk_voxels_loaded, chunk_position, lod) {
      update_chunk(chunk_position, lod);
    }
  }
}

#[inline(never)]
pub fn load_chunk<UpdateView>(
  client: &client::T,
  update_view: &mut UpdateView,
  chunk_position: &chunk_position::T,
  lod: lod::T,
) where
  UpdateView: FnMut(view::update::T),
{
  debug!("generate {:?} at {:?}", chunk_position, lod);
  let voxels = client.voxels.lock().unwrap();
  let mut rng = client.rng.lock().unwrap();
  let mesh_chunk = terrain_mesh::generate(&voxels, &chunk_position, lod, &client.id_allocator, &mut *rng);

  let mut updates = Vec::new();

  use std::collections::hash_map::Entry::{Vacant, Occupied};
  // TODO: Rc instead of clone.
  match client.loaded_chunks.lock().unwrap().entry(*chunk_position) {
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

pub fn lod_index(distance: i32) -> lod::T {
  assert!(distance >= 0);
  let mut lod = 0;
  while
    lod < client::LOD_THRESHOLDS.len()
    && client::LOD_THRESHOLDS[lod] < distance
  {
    lod += 1;
  }
  lod::T(num::traits::FromPrimitive::from_usize(lod).unwrap())
}
