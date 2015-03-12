use env_logger;
use std::env;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Mutex;
use std::thread;
use time;

use common::serialize as binary;
use common::socket::ReceiveSocket;
use common::stopwatch::TimerSet;
use common::terrain_block::{BLOCK_WIDTH, TEXTURE_WIDTH};

use client_recv_thread::apply_client_update;
use opencl_context::CL;
use server::Server;
use terrain::texture_generator::TerrainTextureGenerator;
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
    = args.next().unwrap_or(String::from_str("ipc:///tmp/server.ipc"));
  assert!(args.next().is_none());

  info!("Listening on {}.", listen_url);

  let (listen_thread_send, listen_thread_recv) = channel();
  let (gaia_thread_send, gaia_thread_recv) = channel();

  let listen_thread_recv = Mutex::new(listen_thread_recv);
  let gaia_thread_recv = Mutex::new(gaia_thread_recv);

  let _listen_thread = {
    let listen_thread_send = listen_thread_send.clone();
    thread::scoped(move || {
      let mut socket = ReceiveSocket::new(listen_url.as_slice(), None);
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

  {
    let gaia_thread_send = gaia_thread_send.clone();
    let listen_thread_recv = &listen_thread_recv;
    threads.push(thread::scoped(move || {
      let timers = TimerSet::new();
      let timers = &timers;

      let cl = unsafe {
        CL::new()
      };
      let cl = &cl;

      let texture_generators = [
          TerrainTextureGenerator::new(&cl, TEXTURE_WIDTH[0], BLOCK_WIDTH as u32),
          TerrainTextureGenerator::new(&cl, TEXTURE_WIDTH[1], BLOCK_WIDTH as u32),
          TerrainTextureGenerator::new(&cl, TEXTURE_WIDTH[2], BLOCK_WIDTH as u32),
          TerrainTextureGenerator::new(&cl, TEXTURE_WIDTH[3], BLOCK_WIDTH as u32),
      ];
      let texture_generators = &texture_generators;

      in_series!(
        {
          listen_thread_recv.lock().unwrap().try_recv_opt()
            .map_to_bool(|up| {
              let up = binary::decode(up.as_slice()).unwrap();
              apply_client_update(server, &mut |block| { gaia_thread_send.send(block).unwrap() }, up)
            })
        },
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
          gaia_thread_recv.lock().unwrap().try_recv_opt()
            .map_to_bool(|up| {
              update_gaia(timers, server, texture_generators, cl, up)
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
