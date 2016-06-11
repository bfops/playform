use bincode;
use cgmath;
use cgmath::Point;
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
      update_gaia::Message::Load(
        0,
        vec!(common::voxel::bounds::new(p.x, p.y, p.z, lg_sample_size[lod])),
        update_gaia::LoadReason::Drop,
      )
    })
    .collect();
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

  let mut threads = Vec::new();

  unsafe {
    threads.push(thread_scoped::scoped(|| {
      while !*quit_signal.lock().unwrap() {
        info!("Outstanding gaia updates: {}", gaia_updates.lock().unwrap().len());
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
