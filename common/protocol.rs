//! Defines the messages passed between client and server.

use cgmath::{Vector2, Vector3, Point3};
use collision::{Aabb3};
use std::default::Default;
use std::ops::Add;

use chunk;
use entity_id;
use voxel;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, RustcEncodable, RustcDecodable)]
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

#[derive(Debug, Clone, RustcEncodable, RustcDecodable)]
/// Messages the client sends to the server.
pub enum ClientToServer {
  /// Notify the server that the client exists, and provide a "return address".
  Init(String),
  /// Ping
  Ping(ClientId),
  /// Ask the server to create a new player.
  AddPlayer(ClientId),
  /// Add a vector the player's acceleration.
  Walk(entity_id::T, Vector3<f32>),
  /// Rotate the player by some amount.
  RotatePlayer(entity_id::T, Vector2<f32>),
  /// [Try to] start a jump for the player.
  StartJump(entity_id::T),
  /// [Try to] stop a jump for the player.
  StopJump(entity_id::T),
  #[allow(missing_docs)]
  /// Ask the server to send a block of terrain.
  RequestChunk { requested_at: u64, client_id: ClientId, position: chunk::position::T },
  /// Brush-remove where the player's looking.
  Add(entity_id::T),
  /// Brush-add at where the player's looking.
  Remove(entity_id::T),
}

#[derive(Debug, Clone, RustcEncodable, RustcDecodable)]
/// Collision events. First ID is "collider", rest of IDs are collidee(s).
#[allow(missing_docs)]
pub enum Collision {
  PlayerTerrain(entity_id::T, entity_id::T),
  PlayerMisc(entity_id::T, entity_id::T),
}

#[derive(Debug, Clone, RustcEncodable, RustcDecodable)]
/// Messages the server sends to the client.
pub enum ServerToClient {
  /// Provide the client a unique id to tag its messages.
  LeaseId(ClientId),
  /// Ping
  Ping,

  /// Complete an AddPlayer request.
  PlayerAdded(entity_id::T, Point3<f32>),

  /// Update a player's position.
  UpdatePlayer(entity_id::T, Aabb3<f32>),
  /// Update the client's view of a mob with a given mesh.
  UpdateMob(entity_id::T, Aabb3<f32>),
  /// The sun as a [0, 1) portion of its cycle.
  UpdateSun(f32),

  /// Provide a chunk of terrain to a client.
  #[allow(missing_docs)]
  Chunk { requested_at: u64, chunk: chunk::T },
  /// Send voxel updates to the client.
  #[allow(missing_docs)]
  Voxels { voxels: Vec<(voxel::bounds::T, voxel::T)> },
  /// A collision happened.
  Collision(Collision),
}
