use env_logger;
use std::env;
use std::sync::mpsc::channel;
use std::sync::Mutex;
use std::thread;

use common::socket::ReceiveSocket;
use common::stopwatch::TimerSet;
use common::terrain_block::{BLOCK_WIDTH, TEXTURE_WIDTH};

use client_recv_thread::client_recv_thread;
use opencl_context::CL;
use server::Server;
use terrain::texture_generator::TerrainTextureGenerator;
use update_gaia::update_gaia;
use update_thread::update_thread;

#[main]
fn main() {
  env_logger::init().unwrap();

  let mut args = env::args();
  args.next().unwrap();
  let listen_url
    = args.next().unwrap_or(String::from_str("ipc:///tmp/server.ipc"));
  assert!(args.next().is_none());

  info!("Listening on {}.", listen_url);

  let (socket_sender, incoming) = channel();
  let _listen_thread = {
    thread::scoped(move || {
      let mut socket = ReceiveSocket::new(listen_url.as_slice());
      loop {
        let msg = socket.read();
        socket_sender.send(msg).unwrap();
      }
    })
  };

  let server = Server::new();

  let (gaia_thread_send, gaia_thread_recv) = channel();
  let gaia_thread = {
    let server = &server;

    thread::scoped(move || {
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

      let timers = TimerSet::new();
      let timers = &timers;

      while let Some(update) = gaia_thread_recv.recv().unwrap() {
        update_gaia(
          timers,
          &server,
          &texture_generators,
          cl,
          update,
        );
      }
    })
  };

  let gaia_thread_send = Mutex::new(gaia_thread_send);

  {
    let _update_thread = {
      let server = &server;
      let gaia_thread_send = &gaia_thread_send;
      thread::scoped(move || {
        let timers = TimerSet::new();
        update_thread(
          &timers,
          server,
          &mut |msg| {
            gaia_thread_send.lock().unwrap().send(Some(msg)).unwrap()
          },
        );
      })
    };

    client_recv_thread(
      &server,
      &incoming,
      &mut |msg| {
        gaia_thread_send.lock().unwrap().send(Some(msg)).unwrap()
      },
    );

    gaia_thread_send.lock().unwrap().send(None).unwrap();
    gaia_thread.join();
  }
}
