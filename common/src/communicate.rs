//! Defines the messages passed between client and server.

use cgmath::{Aabb3, Vector2, Vector3, Point3};
use std::default::Default;
use std::ops::Add;

use block_position::BlockPosition;
use entity::EntityId;
use lod::LODIndex;
use serialize::{Copyable, Flatten, MemStream, EOF};
use terrain_block::TerrainBlock;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
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
/// TerrainBlock plus identifying info, e.g. for transmission between server and client.
pub struct TerrainBlockSend {
  #[allow(missing_docs)]
  pub position: Copyable<BlockPosition>,
  #[allow(missing_docs)]
  pub block: TerrainBlock,
  #[allow(missing_docs)]
  pub lod: Copyable<LODIndex>,
}

flatten_struct_impl!(TerrainBlockSend, position, block, lod);

#[derive(Debug, Clone)]
/// Messages the client sends to the server.
pub enum ClientToServer {
  /// Notify the server that the client exists, and provide a "return address".
  Init(String),
  /// Ping
  Ping(Copyable<ClientId>),
  /// Ask the server to create a new player.
  AddPlayer(Copyable<ClientId>),
  /// Add a vector the player's acceleration.
  Walk(Copyable<EntityId>, Copyable<Vector3<f32>>),
  /// Rotate the player by some amount.
  RotatePlayer(Copyable<EntityId>, Copyable<Vector2<f32>>),
  /// [Try to] start a jump for the player.
  StartJump(Copyable<EntityId>),
  /// [Try to] stop a jump for the player.
  StopJump(Copyable<EntityId>),
  /// Ask the server to send a block of terrain.
  RequestBlock(Copyable<ClientId>, Copyable<BlockPosition>, Copyable<LODIndex>),
  /// Remove the voxel the given player's looking at.
  RemoveVoxel(Copyable<EntityId>),
}

flatten_enum_impl!(
  ClientToServer,
  Copyable<u8>,
  (Init, Copyable(0), Copyable(0), x),
  (Ping, Copyable(1), Copyable(1), x),
  (AddPlayer, Copyable(2), Copyable(2), x),
  (Walk, Copyable(3), Copyable(3), x, y),
  (RotatePlayer, Copyable(4), Copyable(4), x, y),
  (StartJump, Copyable(5), Copyable(5), x),
  (StopJump, Copyable(6), Copyable(6), x),
  (RequestBlock, Copyable(7), Copyable(7), x, y, z),
  (RemoveVoxel, Copyable(8), Copyable(8), x),
);

#[derive(Debug, Clone)]
/// Messages the server sends to the client.
pub enum ServerToClient {
  /// Provide the client a unique id to tag its messages.
  LeaseId(Copyable<ClientId>),
  /// Ping
  Ping(Copyable<()>),

  /// Complete an AddPlayer request.
  PlayerAdded(Copyable<EntityId>, Copyable<Point3<f32>>),
  /// Update a player's position.
  UpdatePlayer(Copyable<EntityId>, Copyable<Aabb3<f32>>),

  /// Update the client's view of a mob with a given mesh.
  UpdateMob(Copyable<EntityId>, Copyable<Aabb3<f32>>),

  /// The sun as a [0, 1) portion of its cycle.
  UpdateSun(Copyable<f32>),

  /// Provide a block of terrain to a client.
  UpdateBlock(TerrainBlockSend),
}

flatten_enum_impl!(
  ServerToClient,
  Copyable<u8>,
  (LeaseId, Copyable(0), Copyable(0), x),
  (Ping, Copyable(1), Copyable(1), x),
  (PlayerAdded, Copyable(2), Copyable(2), x, y),
  (UpdatePlayer, Copyable(3), Copyable(3), x, y),
  (UpdateMob, Copyable(4), Copyable(4), x, y),
  (UpdateSun, Copyable(5), Copyable(5), x),
  (UpdateBlock, Copyable(6), Copyable(6), x),
);
