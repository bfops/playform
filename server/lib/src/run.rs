use std;
use std::convert::AsRef;
use std::sync::Mutex;
use bincode;
use stopwatch;
use thread_scoped;
use time;

use common::closure_series;
use common::socket::ReceiveSocket;

use client_recv_thread::apply_client_update;
use server::Server;
use update_gaia;
use update_gaia::update_gaia;
use update_world::update_world;

#[allow(missing_docs)]
pub fn run(listen_url: &str, quit_signal: &Mutex<bool>) {
  let gaia_updates = Mutex::new(std::collections::VecDeque::new());

  let listen_socket = ReceiveSocket::new(listen_url.as_ref(), None);
  let listen_socket = Mutex::new(listen_socket);

  let server = Server::new();
  let server = &server;

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
        quit_upon(&quit_signal),
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
  server: &'a Server,
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
  server: &'a Server,
  mut to_gaia: ToGaia,
) -> closure_series::Closure<'a> where
  ToGaia: FnMut(update_gaia::Message) + 'a,
{
  box move || {
    match socket.lock().unwrap().try_read() {
      None => closure_series::Continue,
      Some(up) => {
        let up = bincode::rustc_serialize::decode(up.as_ref()).unwrap();
        apply_client_update(server, &mut to_gaia, up);
        closure_series::Restart
      },
    }
  }
}

fn consider_gaia_update<'a, Get>(
  server: &'a Server,
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
