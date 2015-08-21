use env_logger;
use std::convert::AsRef;
use std::env;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Mutex;
use std::thread;
use stopwatch::TimerSet;
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
  #[inline(always)]
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
  #[inline(always)]
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

  let (listen_thread_send, listen_thread_recv) = channel();
  let (gaia_thread_send, gaia_thread_recv) = channel();

  let listen_thread_recv = Mutex::new(listen_thread_recv);
  let gaia_thread_recv = Mutex::new(gaia_thread_recv);

  let _listen_thread = unsafe {
    let listen_thread_send = listen_thread_send.clone();
    thread_scoped::scoped(move || {
      let mut socket = ReceiveSocket::new(listen_url.as_ref(), None);
      loop {
        let msg = socket.read();
        listen_thread_send.send(msg).unwrap();
      }
    })
  };

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

  let mut threads = Vec::new();

  unsafe {
    let gaia_thread_send = gaia_thread_send.clone();
    let listen_thread_recv = &listen_thread_recv;
    threads.push(thread_scoped::scoped(move || {
      let timers = TimerSet::new();
      let timers = &timers;

      in_series!(
        {
          if server.update_timer.lock().unwrap().update(time::precise_time_ns()) > 0 {
            update_world(
              timers,
              server,
              &gaia_thread_send,
            );
            true
          } else {
            false
          }
        },
        {
          listen_thread_recv.lock().unwrap().try_recv_opt()
            .map_to_bool(|up| {
              let up = binary::decode(up.as_ref()).unwrap();
              apply_client_update(server, &mut |block| { gaia_thread_send.send(block).unwrap() }, up)
            })
        },
        {
          gaia_thread_recv.lock().unwrap().try_recv_opt()
            .map_to_bool(|up| {
              update_gaia(timers, server, up)
            })
        },
      );
    }));
  }
}

#[test]
fn keep_bin_code_live() {
  let i = 0;
  if i == 1 {
    main();
  }
}
