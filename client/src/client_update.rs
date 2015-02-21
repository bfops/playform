//! Data/code for applying updates to the client from other systems.

use cgmath::{Vector2, Vector3};

#[derive(Clone)]
/// Updates the `View` can send the `Client`.
pub enum ViewToClient {
  /// Add to the player's walking acceleration.
  Walk(Vector3<f32>),
  /// Turn the client laterally and vertically.
  RotatePlayer(Vector2<f32>),
  /// Start the player jumping.
  StartJump,
  /// Stop the player jumping.
  StopJump,
  /// Halt the client.
  Quit,
}
