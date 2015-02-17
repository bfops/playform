//! Defines the messages passed between client and server.

use block_position::BlockPosition;
use entity::EntityId;
use lod::{LODIndex, OwnerId};
use nalgebra::{Vec2, Vec3, Pnt3};
use nanomsg::Socket;
use process_events::{process_channel, process_socket};
use rustc_serialize::{Encodable, Decodable, json};
use std::old_io::timer;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread::Thread;
use std::time::duration::Duration;
use terrain_block::TerrainBlock;
use vertex::ColoredVertex;

#[derive(Debug, Clone)]
#[derive(RustcDecodable, RustcEncodable)]
/// Messages the client sends to the server.
pub enum ClientToServer {
  /// Notify the server that the client exists, and provide a "return address".
  Init(String),
  /// Add a vector the player's acceleration.
  Walk(Vec3<f32>),
  /// Rotate the player by some amount.
  RotatePlayer(Vec2<f32>),
  /// [Try to] start a jump for the player.
  StartJump,
  /// [Try to] stop a jump for the player.
  StopJump,
  /// Ask the server to send a block of terrain.
  RequestBlock(BlockPosition, LODIndex),
  /// Kill the server.
  Quit,
}

#[derive(Debug, Clone)]
#[derive(RustcDecodable, RustcEncodable)]
/// Messages the server sends to the client.
pub enum ServerToClient {
  /// Give the client an OwnerId for terrain load requests.
  LeaseId(OwnerId),

  /// Update the player's position.
  UpdatePlayer(Pnt3<f32>),

  /// Tell the client to add a new mob with the given mesh.
  AddMob(EntityId, Vec<ColoredVertex>),
  /// Update the client's view of a mob with a given mesh.
  UpdateMob(EntityId, Vec<ColoredVertex>),

  /// The sun as a [0, 1) portion of its cycle.
  UpdateSun(f32),

  /// Provide a block of terrain to a client.
  AddBlock(BlockPosition, TerrainBlock, LODIndex),
}

// TODO: This should be a struct SerialSocket<T> or something similar.
// T should not vary for a given socket.
/// Implements a function to serialize & send data.
pub trait SendSerialized {
  /// Serialize and send a request.
  fn send<T>(&mut self, request: T) where T: Encodable;
}

impl SendSerialized for Socket {
  fn send<T>(&mut self, request: T) where T: Encodable {
    let request = json::encode(&request).unwrap();
    if let Err(e) = self.write_all(request.as_bytes()) {
      panic!("Error sending message: {:?}", e);
    }
    if let Err(e) = self.read_to_end() {
      panic!("Error getting ack: {:?}", e);
    }
  }
}

/// Spawn a new thread to send messages to a socket and wait for acks.
pub fn spark_socket_sender<T>(mut socket: Socket) -> Sender<T>
  where T: Send + Encodable
{
  let (send, recv) = channel();

  Thread::spawn(move || {
    loop {
      process_channel(
        &recv,
        |t| {
          socket.send(t);
          true
        },
      );

      timer::sleep(Duration::milliseconds(0));
    }
  });

  send
}

/// Spawn a new thread to read messages from a socket and ack.
pub fn spark_socket_receiver<T>(mut socket: Socket) -> Receiver<T>
  where T: Send + Decodable
{
  let (send, recv) = channel();

  Thread::spawn(move || {
    loop {
      // TODO: This would run faster (i.e. clients would block less)
      // if we sent the raw message and parsed it into T in the receiving thread.
      process_socket(
        &mut socket,
        |t| {
          send.send(t).unwrap();
          true
        },
      );

      timer::sleep(Duration::milliseconds(0));
    }
  });

  recv
}
