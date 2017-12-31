//! Benchmarks for throughput of isosurface extraction.

#![deny(missing_docs)]
#![deny(warnings)]

extern crate common;
extern crate client_lib;
extern crate server_lib;

extern crate cgmath;
extern crate collision;

extern crate env_logger;
#[macro_use]
extern crate log;
extern crate rand;
extern crate time;

mod generate_terrain;

use common::{id_allocator};
use client_lib::{chunk_stats, terrain};
use std::sync::Mutex;

fn main() {
  println!("Loading terrain..");

  let mut terrain = terrain::new(generate_terrain::max_load_distance());

  for voxels in generate_terrain::generate_all_terrain() {
    terrain.enqueue(terrain::Load::Voxels { voxels, time_requested: None });
  }

  println!("Starting..");
  let start = time::precise_time_ns();

  let terrain_allocator = Mutex::new(id_allocator::new());
  let grass_allocator = Mutex::new(id_allocator::new());
  let mut rng: rand::XorShiftRng = rand::SeedableRng::from_seed([1, 2, 3, 4]);
  let mut chunk_stats = chunk_stats::new();

  while terrain.queued_update_count() > 0 {
    terrain.tick(
      &terrain_allocator,
      &grass_allocator,
      &mut rng,
      &mut chunk_stats,
      &mut |_| {},
      &generate_terrain::player_position(),
    );
  }

  let now = time::precise_time_ns();

  println!("Completed in {:.1}s", ((now-start) as f32)/1e9);
}
