use bincode;
use cgmath;
use cgmath::{Aabb, Point};
use std;
use std::convert::AsRef;
use std::sync::Mutex;
use stopwatch;
use thread_scoped;
use time;

use common;
use common::id_allocator;
use common::closure_series;
use common::socket::ReceiveSocket;
use common::voxel;

use client_recv_thread::apply_client_update;
use player;
use server;
use update_gaia;
use update_gaia::update_gaia;
use update_world::update_world;

mod terrain {
  pub use ::terrain::*;
}

fn center(bounds: &cgmath::Aabb3<f32>) -> cgmath::Point3<f32> {
  bounds.min.add_v(&bounds.max.to_vec()).mul_s(1.0 / 2.0)
}

pub fn voxels_in(bounds: &cgmath::Aabb3<i32>, lg_size: i16) -> Vec<voxel::bounds::T> {
  let delta = bounds.max().sub_p(bounds.min());

  // assert that lg_size samples fit neatly into the bounds.
  let mod_size = (1 << lg_size) - 1;
  assert!(bounds.min().x & mod_size == 0);
  assert!(bounds.min().y & mod_size == 0);
  assert!(bounds.min().z & mod_size == 0);
  assert!(bounds.max().x & mod_size == 0);
  assert!(bounds.max().y & mod_size == 0);
  assert!(bounds.max().z & mod_size == 0);

  let x_len = delta.x >> lg_size;
  let y_len = delta.y >> lg_size;
  let z_len = delta.z >> lg_size;

  let x_off = bounds.min().x >> lg_size;
  let y_off = bounds.min().y >> lg_size;
  let z_off = bounds.min().z >> lg_size;

  let mut voxels =
    Vec::with_capacity(x_len as usize + y_len as usize + z_len as usize);

  for dx in 0 .. x_len {
  for dy in 0 .. y_len {
  for dz in 0 .. z_len {
    let x = x_off + dx;
    let y = y_off + dy;
    let z = z_off + dz;
    voxels.push(voxel::bounds::new(x, y, z, lg_size));
  }}}
  voxels
}

#[allow(missing_docs)]
pub fn run(listen_url: &str, quit_signal: &Mutex<bool>) {
  let lod_thresholds = vec!(2, 16, 32);
  let lg_edge_samples = vec!(3, 2, 1, 0);
  let lg_sample_size: Vec<_> = lg_edge_samples.iter().map(|x| 3 - *x as i16).collect();
  let gaia_updates: std::collections::VecDeque<_> =
    common::surroundings_loader::SurroundingsLoader::new(80, lod_thresholds.clone())
    .updates(&cgmath::Point3::new(0, 64, 0))
    .map(|(p, _)| {
      let distance = std::cmp::min(std::cmp::min(p.x, p.y), p.z);
      let mut lod = 0;
      while lod < lod_thresholds.len() && distance >= lod_thresholds[lod] {
        lod += 1;
      }
      let voxel_size = 1 << lg_sample_size[lod];
      let bounds =
        cgmath::Aabb3::new(
          cgmath::Point3::new(
            p.x << 3,
            p.y << 3,
            p.z << 3,
          ),
          cgmath::Point3::new(
            (p.x + 1) << 3,
            (p.y + 1) << 3,
            (p.z + 1) << 3,
          ),
        );
      update_gaia::Message::Load(
        0,
        voxels_in(&bounds, lg_sample_size[lod]),
        update_gaia::LoadReason::Drop,
      )
    })
    .collect();
  let total = gaia_updates.len();
  let gaia_updates = Mutex::new(gaia_updates);

  let server = server::new();
  let server = &server;

  let terrain_path = std::path::Path::new("default.terrain");

  println!("Loading terrain from {}", terrain_path.to_str().unwrap());
  load_terrain(&server.terrain_loader.terrain, &terrain_path);

  {
    let client_url = "ipc:///dev/null";

    let mut client =
      server::Client {
        socket: common::socket::SendSocket::new(client_url, Some(std::time::Duration::from_secs(30))),
      };

    let client_id = id_allocator::allocate(&server.client_allocator);

    server.clients.lock().unwrap().insert(client_id, client);

    let mut player =
      player::T::new(
        id_allocator::allocate(&server.id_allocator),
        &server.owner_allocator,
      );

    // TODO: shift upward until outside terrain
    let min: cgmath::Point3<f32> = cgmath::Point3::new(0.0, 64.0, 4.0);
    let max = min.add_v(&cgmath::Vector3::new(1.0, 2.0, 1.0));
    let bounds = cgmath::Aabb3::new(min, max);
    server.physics.lock().unwrap().insert_misc(player.entity_id, &bounds);

    player.position = center(&bounds);
    player.rotate_lateral(std::f32::consts::PI / 2.0);

    let id = player.entity_id;
    let pos = player.position;

    server.players.lock().unwrap().insert(id, player);
  }

  let start = time::precise_time_ns() as f32;

  let mut threads = Vec::new();

  unsafe {
    threads.push(thread_scoped::scoped(|| {
      while !*quit_signal.lock().unwrap() {
        let len = gaia_updates.lock().unwrap().len();
        info!("Outstanding gaia updates: {}", len);
        let delta_t = (time::precise_time_ns() as f32 - start) / 1e9;
        let delta_n = total - len;
        let rate = delta_n as f32 / delta_t;
        info!("Rate: {} Hz", rate);
        info!("ETA: {} s", len as f32 / rate);
        info!("ETA Total: {} m", total as f32 / rate / 60.0);
        std::thread::sleep(std::time::Duration::from_secs(1));
      }

      stopwatch::clone()
    }))
  }

  unsafe {
    let server = &server;
    let gaia_updates = &gaia_updates;
    threads.push(thread_scoped::scoped(move || {
      closure_series::new(vec!(
        quit_upon(quit_signal, &gaia_updates),
        consider_world_update(&server, |up| { gaia_updates.lock().unwrap().push_back(up) }),
        consider_gaia_update(&server, || { gaia_updates.lock().unwrap().pop_front() } ),
      ))
      .until_quit();

      stopwatch::clone()
    }));
  }

  for thread in threads.into_iter() {
    let stopwatch = thread.join();
    stopwatch.print();
  }

  println!("Saving terrain to {}", terrain_path.to_str().unwrap());
  stopwatch::time("save_terrain", || {
    save_terrain(&server.terrain_loader.terrain, &terrain_path);
  });

  stopwatch::clone().print();
}

fn quit_upon<'a, T>(quit_signal: &'a Mutex<bool>, queue: &'a Mutex<std::collections::VecDeque<T>>) -> closure_series::Closure<'a> {
  box move || {
    if queue.lock().unwrap().is_empty() {
      *quit_signal.lock().unwrap() = true;
      closure_series::Quit
    } else {
      closure_series::Continue
    }
  }
}

fn consider_world_update<'a, ToGaia>(
  server: &'a server::T,
  mut to_gaia: ToGaia,
) -> closure_series::Closure<'a> where
  ToGaia: FnMut(update_gaia::Message) + 'a,
{
  box move || {
    if server.update_timer.lock().unwrap().update(time::precise_time_ns()) > 0 {
      update_world(
        server,
        &mut to_gaia,
      );
      closure_series::Restart
    } else {
      closure_series::Continue
    }
  }
}

fn consider_gaia_update<'a, Get>(
  server: &'a server::T,
  mut get_update: Get,
) -> closure_series::Closure<'a> where
  Get: FnMut() -> Option<update_gaia::Message> + 'a,
{
  box move || {
    match get_update() {
      Some(up) => {
        update_gaia(server, up);
        closure_series::Restart
      },
      None => closure_series::Continue,
    }
  }
}

fn load_terrain(terrain: &terrain::T, path: &std::path::Path) {
  let mut file =
    match std::fs::File::open(path) {
      Err(err) => {
        warn!("Error opening terrain file: {:?}", err);
        return
      },
      Ok(file) => file,
    };
  let loaded =
    bincode::rustc_serialize::decode_from(
      &mut file,
      bincode::SizeLimit::Infinite,
    );
  let loaded =
    match loaded {
      Ok(loaded) => loaded,
      Err(err) => {
        warn!("Error loading terrain: {:?}", err);
        return
      },
    };
  *terrain.voxels.lock().unwrap() = loaded;
}

fn save_terrain(terrain: &terrain::T, path: &std::path::Path) {
  let mut file = std::fs::File::create(path).unwrap();
  bincode::rustc_serialize::encode_into(
    &*terrain.voxels.lock().unwrap(),
    &mut file,
    bincode::SizeLimit::Infinite,
  ).unwrap();
}
