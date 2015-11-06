use bincode;
use nanomsg;
use std;
use std::convert::AsRef;
use std::sync::mpsc::{channel, Sender, Receiver, TryRecvError};
use std::sync::Mutex;
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
pub fn run(listen_url: &str) {
  let (gaia_send, gaia_recv) = channel();

  let gaia_recv = Mutex::new(gaia_recv);

  let listen_socket = ReceiveSocket::new(listen_url.as_ref(), None);
  let listen_socket = Mutex::new(listen_socket);

  let server = Server::new();
  let server = &server;

  let quit_signal = Mutex::new(false);

  let mut threads = Vec::new();

  unsafe {
    let server = &server;
    let gaia_send = gaia_send.clone();
    let gaia_recv = &gaia_recv;
    let quit_signal = &quit_signal;
    let listen_socket = &listen_socket;
    threads.push(thread_scoped::scoped(move || {
      closure_series::new(vec!(
        quit_upon(&quit_signal),
        consider_world_update(&server, gaia_send.clone()),
        network_listen(&listen_socket, server, gaia_send.clone()),
        consider_gaia_update(&server, &gaia_recv),
      ))
      .until_quit();

      stopwatch::clone()
    }));
  }
  unsafe {
    let server = &server;
    let gaia_send = gaia_send.clone();
    let quit_signal = &quit_signal;
    let listen_socket = &listen_socket;
    threads.push(thread_scoped::scoped(move || {
      closure_series::new(vec!(
        quit_upon(&quit_signal),
        consider_world_update(&server, gaia_send.clone()),
        network_listen(&listen_socket, server, gaia_send.clone()),
      ))
      .until_quit();

      stopwatch::clone()
    }));
  }

  unsafe {
    let quit_signal = &quit_signal;
    threads.push(thread_scoped::scoped(move || {
      closure_series::new(vec!(
        wait_for_quit(quit_signal),
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

fn consider_world_update(
  server: &Server, 
  to_gaia: Sender<update_gaia::Message>,
) -> closure_series::Closure {
  box move || {
    if server.update_timer.lock().unwrap().update(time::precise_time_ns()) > 0 {
      update_world(
        server,
        &to_gaia,
      );
      closure_series::Restart
    } else {
      closure_series::Continue
    }
  }
}

fn network_listen<'a>(
  socket: &'a Mutex<ReceiveSocket>, 
  server: &'a Server, 
  to_gaia: Sender<update_gaia::Message>,
) -> closure_series::Closure<'a> {
  box move || {
    match socket.lock().unwrap().try_read() {
      None => closure_series::Continue,
      Some(up) => {
        let up = bincode::rustc_serialize::decode(up.as_ref()).unwrap();
        apply_client_update(server, &mut |block| { to_gaia.send(block).unwrap() }, up);
        closure_series::Restart
      },
    }
  }
}

fn consider_gaia_update<'a>(
  server: &'a Server, 
  to_gaia: &'a Mutex<Receiver<update_gaia::Message>>,
) -> closure_series::Closure<'a> {
  box move || {
    match to_gaia.lock().unwrap().try_recv() {
      Ok(up) => {
        update_gaia(server, up);
        closure_series::Restart
      },
      Err(TryRecvError::Empty) => closure_series::Continue,
      err => {
        err.unwrap();
        unreachable!();
      },
    }
  }
}

fn wait_for_quit(
  quit_signal: &Mutex<bool>,
) -> closure_series::Closure {
  box move || {
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();

    if line == "quit\n" {
      println!("Quitting");
      *quit_signal.lock().unwrap() = true;

      // Close all sockets.
      nanomsg::Socket::terminate();

      closure_series::Quit
    } else {
      println!("Unrecognized command: {:?}", line);
      closure_series::Continue
    }
  }
}
