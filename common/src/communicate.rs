//! Defines the messages passed between client and server.

use block_position::BlockPosition;
use entity::EntityId;
use lod::LODIndex;
use cgmath::{Vector2, Vector3, Point3};
use rustc_serialize::Encodable;
use vertex::ColoredVertex;

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
