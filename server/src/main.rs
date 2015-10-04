use env_logger;
use nanomsg;
use std;
use std::io::Read;
use std::convert::AsRef;
use std::env;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Mutex;
use std::thread;
use stopwatch;
use thread_scoped;
use time;

use common::serialize as binary;
use common::socket::ReceiveSocket;

use client_recv_thread::apply_client_update;
use server::Server;
use update_gaia::update_gaia;
use update_world::update_world;

// TODO: This is duplicated in the client. Fix that.
#[allow(missing_docs)]
trait TryRecv<T> {
  #[allow(missing_docs)]
  fn try_recv_opt(&self) -> Option<T>;
}

impl<T> TryRecv<T> for Receiver<T> where T: Send {
  fn try_recv_opt(&self) -> Option<T> {
    match self.try_recv() {
      Ok(msg) => Some(msg),
      Err(TryRecvError::Empty) => None,
      e => Some(e.unwrap()),
    }
  }
}

#[allow(missing_docs)]
trait MapToBool<T> {
  #[allow(missing_docs)]
  fn map_to_bool<F: FnOnce(T)>(self, f: F) -> bool;
}

impl<T> MapToBool<T> for Option<T> {
  fn map_to_bool<F: FnOnce(T)>(self, f: F) -> bool {
    match self {
      None => false,
      Some(t) => {
        f(t);
        true
      },
    }
  }
}

#[main]
fn main() {
  env_logger::init().unwrap();

  let mut args = env::args();
  args.next().unwrap();
  let listen_url
    = args.next().unwrap_or(String::from("ipc:///tmp/server.ipc"));
  assert!(args.next().is_none());

  info!("Listening on {}.", listen_url);

  let (gaia_thread_send, gaia_thread_recv) = channel();

  let gaia_thread_recv = Mutex::new(gaia_thread_recv);

  let listen_socket = ReceiveSocket::new(listen_url.as_ref(), None);
  let listen_socket = Mutex::new(listen_socket);

  let server = Server::new();
  let server = &server;

  // Add a thread that performs several actions repeatedly in a prioritized order:
  // Only if an action fails do we try the next action; otherwise, we restart the chain.
  macro_rules! in_series(
    ( $($action: expr,)* ) => {
      loop {
        $(
          if $action {
            continue
          }
        )*

        thread::yield_now();
      }
    };
  );

  let quit_upon = Mutex::new(false);

  let mut threads = Vec::new();

  unsafe {
    let gaia_thread_send = gaia_thread_send.clone();
    let quit_upon = &quit_upon;
    let listen_socket = &listen_socket;
    threads.push(thread_scoped::scoped(move || {
      in_series!(
        {
          if *quit_upon.lock().unwrap() {
            return stopwatch::clone()
          }
          false
        },
        {
          if server.update_timer.lock().unwrap().update(time::precise_time_ns()) > 0 {
            update_world(
              server,
              &gaia_thread_send,
            );
            true
          } else {
            false
          }
        },
        {
          listen_socket.lock().unwrap().try_read()
            .map_to_bool(|up| {
              let up = binary::decode(up.as_ref()).unwrap();
              apply_client_update(server, &mut |block| { gaia_thread_send.send(block).unwrap() }, up)
            })
        },
        {
          gaia_thread_recv.lock().unwrap().try_recv_opt()
            .map_to_bool(|up| {
              update_gaia(server, up)
            })
        },
      );
    }));
  }
  unsafe {
    let gaia_thread_send = gaia_thread_send.clone();
    let quit_upon = &quit_upon;
    let listen_socket = &listen_socket;
    threads.push(thread_scoped::scoped(move || {
      in_series!(
        {
          if *quit_upon.lock().unwrap() {
            return stopwatch::clone()
          }
          false
        },
        {
          if server.update_timer.lock().unwrap().update(time::precise_time_ns()) > 0 {
            update_world(
              server,
              &gaia_thread_send,
            );
            true
          } else {
            false
          }
        },
        {
          listen_socket.lock().unwrap().try_read()
            .map_to_bool(|up| {
              let up = binary::decode(up.as_ref()).unwrap();
              apply_client_update(server, &mut |block| { gaia_thread_send.send(block).unwrap() }, up)
            })
        },
      );
    }));
  }
  unsafe {
    let quit_upon = &quit_upon;
    threads.push(thread_scoped::scoped(move || {
      loop {
        let mut line = String::new();
        for c in std::io::stdin().chars() {
          let c = c.unwrap();
          if c == '\n' {
            break
          }
          line.push(c);
        }
        if line == "quit" {
          println!("Quitting");
          *quit_upon.lock().unwrap() = true;

          // Close all sockets.
          nanomsg::Socket::terminate();

          return stopwatch::clone()
        } else {
          println!("Unrecognized command: {:?}", line);
        }
      }
    }));
  }

  for thread in threads.into_iter() {
    let stopwatch = thread.join();
    stopwatch.print();
  }

  stopwatch::clone().print();
}
