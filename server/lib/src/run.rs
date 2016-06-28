use std;
use std::convert::AsRef;
use std::sync::Mutex;
use bincode;
use stopwatch;
use thread_scoped;
use time;
use voxel_data;

use common;
use common::closure_series;
use common::socket::ReceiveSocket;

use client_recv_thread::apply_client_update;
use server;
use update_gaia;
use update_gaia::update_gaia;
use update_world::update_world;

mod terrain {
  pub use ::terrain::*;
}

#[allow(missing_docs)]
pub fn run(listen_url: &str, quit_signal: &Mutex<bool>) {
  let gaia_updates = Mutex::new(std::collections::VecDeque::new());

  let listen_socket = ReceiveSocket::new(listen_url.as_ref(), None);
  let listen_socket = Mutex::new(listen_socket);

  let server = server::new();
  let server = &server;

  let terrain_path = std::path::Path::new("default.terrain");

  println!("Loading terrain from {}", terrain_path.to_str().unwrap());
  load_terrain(&server.terrain_loader.terrain, &terrain_path);

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
    let listen_socket = &listen_socket;
    threads.push(thread_scoped::scoped(move || {
      closure_series::new(vec!(
        quit_upon(&quit_signal),
        consider_world_update(&server, |up| { gaia_updates.lock().unwrap().push_back(up) }),
        network_listen(&listen_socket, server, |up| { gaia_updates.lock().unwrap().push_back(up) }),
        consider_gaia_update(&server, || { gaia_updates.lock().unwrap().pop_front() } ),
      ))
      .until_quit();

      stopwatch::clone()
    }));
  }
  unsafe {
    let server = &server;
    let gaia_updates = &gaia_updates;
    let quit_signal = &quit_signal;
    let listen_socket = &listen_socket;
    threads.push(thread_scoped::scoped(move || {
      closure_series::new(vec!(
        quit_upon(&quit_signal),
        consider_world_update(&server, |up| { gaia_updates.lock().unwrap().push_back(up) }),
        network_listen(&listen_socket, server, |up| { gaia_updates.lock().unwrap().push_back(up) }),
      ))
      .until_quit();

      stopwatch::clone()
    }));
  }

  for thread in threads.into_iter() {
    let stopwatch = thread.join();
    stopwatch.print();
  }

  info!("Voxel takes {} bytes", std::mem::size_of::<common::voxel::T>());

  println!(
    "Terrain is using {} MB",
    tree_ram_usage(&server.terrain_loader.terrain.voxels.lock().unwrap()) as f32 / (1 << 20) as f32,
  );

  println!("Saving terrain to {}", terrain_path.to_str().unwrap());
  stopwatch::time("save_terrain", || {
    save_terrain(&server.terrain_loader.terrain, &terrain_path);
  });

  stopwatch::clone().print();
}

fn quit_upon(signal: &Mutex<bool>) -> closure_series::Closure {
  box move || {
    if *signal.lock().unwrap() {
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

fn network_listen<'a, ToGaia>(
  socket: &'a Mutex<ReceiveSocket>,
  server: &'a server::T,
  mut to_gaia: ToGaia,
) -> closure_series::Closure<'a> where
  ToGaia: FnMut(update_gaia::Message) + 'a,
{
  box move || {
    match socket.lock().unwrap().try_read() {
      common::socket::Result::Empty => closure_series::Continue,
      common::socket::Result::Terminating => closure_series::Quit,
      common::socket::Result::Success(up) => {
        let up = bincode::rustc_serialize::decode(up.as_ref()).unwrap();
        apply_client_update(server, &mut to_gaia, up);
        closure_series::Restart
      },
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

fn tree_ram_usage(tree: &common::voxel::tree::T) -> usize {
  fn tree_ram_usage_inner(branches: &common::voxel::tree::Branches, size: &mut usize) {
    *size += std::mem::size_of_val(branches);
    for node in branches.as_flat_array() {
      match node.next {
        voxel_data::tree::Inner::Empty => {},
        voxel_data::tree::Inner::Branches(ref branches) => {
          tree_ram_usage_inner(branches, size);
        },
      }
    }
  }

  let mut r = 0;
  tree_ram_usage_inner(&tree.contents, &mut r);
  r
}
