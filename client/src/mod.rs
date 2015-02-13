//! This crate contains client-only components of Playform.

#![deny(missing_docs)]
#![deny(warnings)]

#![feature(core)]
#![feature(collections)]
#![feature(io)]
#![feature(path)]
#![feature(slicing_syntax)]
#![feature(std_misc)]
#![feature(test)]
#![feature(unboxed_closures)]
#![feature(unsafe_destructor)]

extern crate common;
extern crate gl;
#[macro_use]
extern crate log;
extern crate libc;
extern crate nalgebra;
extern crate sdl2;
extern crate "sdl2-sys" as sdl2_sys;
extern crate test;
extern crate time;
extern crate yaglw;

mod camera;
mod client;
mod client_thread;
mod client_update;
mod fontloader;
mod hud;
mod light;
mod mob_buffers;
mod process_event;
mod render;
mod shaders;
mod terrain_buffers;
mod ttf;
mod view;
mod view_thread;
mod view_update;

use client_thread::client_thread;
use common::communicate::{ServerToClient, ClientToServer};
use common::id_allocator::IdAllocator;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread::Thread;
use view_thread::view_thread;

/// Entry point.
pub fn main(
  ups_from_server: Receiver<ServerToClient>,
  ups_to_server: Sender<ClientToServer>,
) {
  let (view_to_client_send, view_to_client_recv) = channel();
  let (client_to_view_send, client_to_view_recv) = channel();

  let mut owner_alloc = IdAllocator::new();

  let client_id = owner_alloc.allocate();

  let _client_thread =
    Thread::spawn(move ||
      client_thread(
        client_id,
        &ups_from_server,
        &ups_to_server,
        &view_to_client_recv,
        &client_to_view_send,
      )
    );

  view_thread(
    &client_to_view_recv,
    &view_to_client_send,
  );
}
