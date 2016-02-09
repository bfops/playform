//! Define the updates passed from the client to the view.

use cgmath::Point3;
use stopwatch;

use common::color::Color3;
use common::entity_id;

use light;
use light::{set_sun, set_ambient_light};
use mob_buffers::VERTICES_PER_MOB;
use player_buffers::VERTICES_PER_PLAYER;
use terrain_mesh;
use vertex::ColoredVertex;
use view;

pub use self::T::*;

/// Messages from the client to the view.
pub enum T {
  /// Set the camera location.
  MoveCamera(Point3<f32>),

  /// Update a player mesh.
  UpdatePlayer(entity_id::T, [ColoredVertex; VERTICES_PER_PLAYER]),
  /// Update a mob mesh.
  UpdateMob(entity_id::T, [ColoredVertex; VERTICES_PER_MOB]),

  /// Update the sun.
  SetSun(light::Sun),
  /// Update the ambient light.
  SetAmbientLight(Color3<f32>),
  /// Update the GL clear color.
  SetClearColor(Color3<f32>),

  /// Add a terrain block to the view.
  AddBlock(terrain_mesh::T),
  /// Remove a terrain entity.
  RemoveTerrain(entity_id::T),
  /// Treat a series of updates as an atomic operation.
  Atomic(Vec<T>),
}

unsafe impl Send for T {}

#[allow(missing_docs)]
pub fn apply_client_to_view(view: &mut view::T, up: T) {
  match up {
    T::MoveCamera(position) => {
      view.camera.translate_to(position);
    },
    T::UpdateMob(id, triangles) => {
      view.mob_buffers.insert(&mut view.gl, id, &triangles);
    },
    T::UpdatePlayer(id, triangles) => {
      view.player_buffers.insert(&mut view.gl, id, &triangles);
    },
    T::SetSun(sun) => {
      set_sun(
        &mut view.shaders.terrain_shader.shader,
        &mut view.gl,
        &sun,
      );
    },
    T::SetAmbientLight(color) => {
      set_ambient_light(
        &mut view.shaders.terrain_shader.shader,
        &mut view.gl,
        color,
      );
    },
    T::SetClearColor(color) => {
      view.gl.set_background_color(color.r, color.g, color.b, 1.0);
    },
    T::AddBlock(block) => {
      stopwatch::time("add_block", || {
        view.terrain_buffers.push(
          &mut view.gl,

          block.vertex_coordinates.as_ref(),
          block.normals.as_ref(),
          block.ids.as_ref(),
          block.materials.as_ref(),
        );
      })
    },
    T::RemoveTerrain(id) => {
      view.terrain_buffers.swap_remove(&mut view.gl, id);
    },
    T::Atomic(updates) => {
      for up in updates.into_iter() {
        apply_client_to_view(view, up);
      }
    },
  };
}
