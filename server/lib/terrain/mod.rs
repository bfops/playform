//! This crate contains the terrain data structures and generation.

#![allow(let_and_return)]
#![allow(match_ref_pats)]
#![allow(type_complexity)]
#![allow(unneeded_field_pattern)]
#![allow(derive_hash_xor_eq)]

#![deny(missing_docs)]
#![deny(warnings)]

#![feature(main)]
#![feature(plugin)]
#![feature(test)]
#![feature(unboxed_closures)]

#![plugin(clippy)]

extern crate cgmath;
extern crate collision;
extern crate common;
extern crate fnv;
#[macro_use]
extern crate log;
extern crate lru_cache;
extern crate noise;
extern crate rand;
extern crate stopwatch;
extern crate test;
extern crate time;
extern crate voxel_data;
extern crate num;

mod cache_mosaic;

pub mod biome;
pub mod chunk;
pub mod tree;

pub use noise::Seed;

use std::sync::Mutex;

use common::voxel;

/// This struct contains and lazily generates the world's terrain.
#[allow(missing_docs)]
pub struct T {
  pub mosaic: Mutex<cache_mosaic::T<voxel::Material>>,
  pub voxels: Mutex<voxel::tree::T>,
}

impl T {
  #[allow(missing_docs)]
  pub fn new(terrain_seed: Seed) -> T {
    T {
      mosaic: Mutex::new(cache_mosaic::new(Box::new(biome::demo::new(terrain_seed)))),
      voxels: Mutex::new(voxel::tree::new()),
    }
  }

  /// Load the block of terrain at a given position.
  // TODO: Allow this to be performed in such a way that self is only briefly locked.
  pub fn load(
    &self,
    bounds: &voxel::bounds::T,
  ) -> voxel::T {
    let mut voxels = self.voxels.lock().unwrap();
    let node = voxels.get_mut_or_create(bounds);
    match node.data {
      None => {
        let mut mosaic = self.mosaic.lock().unwrap();
        let voxel = voxel::unwrap(voxel::of_field(&mut *mosaic, bounds));
        let r = voxel;
        node.data = Some(voxel);
        r
      },
      Some(data) => {
        data
      },
    }
  }

  /// Apply a voxel brush to the terrain.
  pub fn brush<VoxelChanged, Mosaic>(
    &self,
    brush: &mut voxel::brush::T<Mosaic>,
    mut voxel_changed: VoxelChanged,
  ) where
    VoxelChanged: FnMut(&voxel::T, &voxel::bounds::T),
    Mosaic: voxel::mosaic::T<voxel::Material>,
  {
    let mut voxels = self.voxels.lock().unwrap();
    voxels.brush(
      brush,
      // TODO: Put a max size on this
      &mut |bounds| {
        if bounds.lg_size > 3 {
          None
        } else {
          let mut mosaic = self.mosaic.lock().unwrap();
          Some(voxel::unwrap(voxel::of_field(&mut *mosaic, bounds)))
        }
      },
      &mut voxel_changed,
    );
  }
}
