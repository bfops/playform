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
extern crate time;

use common::surroundings_loader;
use common::surroundings_loader::LoadType;
use client_lib::{chunk, lod, terrain_mesh};
use server_lib::{server, update_gaia};
use update_gaia::LoadDestination;

fn main() {
  println!("Loading terrain..");
  for voxels in generate_all_terrain() {
    terrain.enqueue(voxels);
  }

  println!("Starting..");
  let start = time::precise_time_ns();

  let now = time::precise_time_ns();

  println!("Completed in {:.1}s", ((now-start) as f32)/1e9);
}
