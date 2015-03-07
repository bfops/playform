//! Defines the messages passed between client and server.

use cgmath::{Vector2, Vector3, Point3};
use rustc_serialize::Encodable;
use std::default::Default;
use std::ops::Add;

use block_position::BlockPosition;
use entity::EntityId;
use lod::LODIndex;
use vertex::ColoredVertex;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[derive(RustcDecodable, RustcEncodable)]
/// Unique client ID.
pub struct ClientId(u32);

impl Default for ClientId {
  fn default() -> ClientId {
    ClientId(0)
  }
}

impl Add<u32> for ClientId {
  type Output = ClientId;

  fn add(self, rhs: u32) -> ClientId {
    let ClientId(i) = self;
    ClientId(i + rhs)
  }
}

#[derive(Debug, Clone)]
#[derive(RustcDecodable, RustcEncodable)]
/// TerrainBlock plus identifying info, e.g. for transmission between server and client.
pub struct TerrainBlockSend {
  #[allow(missing_docs)]
  pub position: BlockPosition,
  /// The String-serialized `TerrainBlock`.
  pub block: String,
  #[allow(missing_docs)]
  pub lod: LODIndex,
}

#[derive(Debug, Clone)]
#[derive(RustcDecodable, RustcEncodable)]
/// Messages the client sends to the server.
pub enum ClientToServer {
  /// Notify the server that the client exists, and provide a "return address".
  Init(String),
  /// Add a vector the player's acceleration.
  Walk(Vector3<f32>),
  /// Rotate the player by some amount.
  RotatePlayer(Vector2<f32>),
  /// [Try to] start a jump for the player.
  StartJump,
  /// [Try to] stop a jump for the player.
  StopJump,
  /// Ask the server to send a block of terrain.
  RequestBlock(BlockPosition, LODIndex),
}

#[derive(Debug, Clone)]
#[derive(RustcDecodable, RustcEncodable)]
/// Messages the server sends to the client.
pub enum ServerToClient {
  /// Provide the client a unique id to tag its messages.
  LeaseId(ClientId),

  /// Update the player's position.
  UpdatePlayer(Point3<f32>),

  /// Tell the client to add a new mob with the given mesh.
  AddMob(EntityId, Vec<ColoredVertex>),
  /// Update the client's view of a mob with a given mesh.
  UpdateMob(EntityId, Vec<ColoredVertex>),

  /// The sun as a [0, 1) portion of its cycle.
  UpdateSun(f32),

  /// Provide a block of terrain to a client.
  AddBlock(TerrainBlockSend),
}
