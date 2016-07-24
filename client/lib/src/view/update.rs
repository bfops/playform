//! Define the updates passed from the client to the view.

use cgmath::Point3;
use stopwatch;

use common::entity_id;

use lod;
use terrain_mesh;
use vertex::ColoredVertex;
use view;
use view::light;
use view::mob_buffers::VERTICES_PER_MOB;
use view::player_buffers::VERTICES_PER_PLAYER;

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

  /// Add a terrain block to the view.
  LoadMesh(terrain_mesh::T),
  /// Remove a terrain entity.
  RemoveTerrain(entity_id::T),
  /// Remove a grass billboard.
  RemoveGrass(entity_id::T),
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
      match view.input_mode {
        view::InputMode::Sun => {},
        _ => {
          view.sun = sun;
        },
      }
    },
    T::LoadMesh(mesh) => {
      stopwatch::time("load_mesh", || {
        view.terrain_buffers.push(
          &mut view.gl,
          mesh.vertex_coordinates.as_ref(),
          mesh.normals.as_ref(),
          mesh.ids.as_ref(),
          mesh.materials.as_ref(),
        );
        view.grass_buffers.push(
          &mut view.gl,
          mesh.grass.as_ref(),
          mesh.grass_ids.as_ref(),
        );
      })
    },
    T::RemoveTerrain(id) => {
      view.terrain_buffers.swap_remove(&mut view.gl, id);
    },
    T::RemoveGrass(id) => {
      view.grass_buffers.swap_remove(&mut view.gl, id);
    },
    T::Atomic(updates) => {
      for up in updates.into_iter() {
        apply_client_to_view(view, up);
      }
    },
  };
}
