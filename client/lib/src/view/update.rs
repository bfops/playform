//! Define the updates passed from the client to the view.

use cgmath::Point3;
use stopwatch;

use common::entity_id;

use terrain_mesh;
use vertex::ColoredVertex;
use view;
use view::light;
use view::mob_buffers::VERTICES_PER_MOB;
use view::player_buffers::VERTICES_PER_PLAYER;

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

  /// Add a terrain chunk to the view.
  LoadChunk {
    mesh: terrain_mesh::T,
  },
  /// Remove a terrain entity.
  UnloadChunk {
    ids: terrain_mesh::Ids,
  },
  /// Treat a series of updates as an atomic operation.
  Atomic(Vec<T>),
}

unsafe impl Send for T {}

pub use self::T::*;

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
    T::LoadChunk { mesh } => {
      stopwatch::time("add_chunk", || {
        view.terrain_buffers.push(
          &mut view.gl,
          mesh.vertex_coordinates.as_ref(),
          mesh.normals.as_ref(),
          mesh.ids.terrain_ids.as_ref(),
          mesh.materials.as_ref(),
        );
        view.grass_buffers.push(
          &mut view.gl,
          mesh.grass.as_ref(),
          mesh.ids.grass_ids.as_ref(),
        );
      })
    },
    T::UnloadChunk { ids: terrain_mesh::Ids { terrain_ids, grass_ids } } => {
      for id in terrain_ids {
        view.terrain_buffers.swap_remove(&mut view.gl, id);
      }
      for id in grass_ids {
        view.grass_buffers.swap_remove(&mut view.gl, id);
      }
    },
    T::Atomic(updates) => {
      for up in updates.into_iter() {
        apply_client_to_view(view, up);
      }
    },
  };
}
