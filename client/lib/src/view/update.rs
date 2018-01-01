//! Define the updates passed from the client to the view.

use cgmath::Point3;
use stopwatch;

use terrain_mesh;
use vertex::ColoredVertex;
use view;

use common::index;

use super::chunked_terrain;
use super::entity;
use super::light;
use super::mob_buffers::VERTICES_PER_MOB;
use super::player_buffers::VERTICES_PER_PLAYER;

/// Messages from the client to the view.
pub enum T {
  /// Set the camera location.
  MoveCamera(Point3<f32>),

  /// Update a player mesh.
  UpdatePlayer(entity::id::Player, [ColoredVertex; VERTICES_PER_PLAYER]),
  /// Update a mob mesh.
  UpdateMob(entity::id::Mob, [ColoredVertex; VERTICES_PER_MOB]),

  /// Update the sun.
  SetSun(light::Sun),

  /// Add a terrain chunk to the view.
  LoadMesh (Box<chunked_terrain::T>),
  /// Remove a terrain entity.
  UnloadMesh(terrain_mesh::Ids),
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
    T::LoadMesh(mesh) => {
      stopwatch::time("add_chunk", move || {
        let mesh = *mesh;
        for i in 0 .. mesh.chunk_count() {
          view.terrain_buffers.push(
            &mut view.gl,
            mesh.ids[i],
            &mesh.vertex_coordinates[i],
            &mesh.normals[i],
            &mesh.materials[i],
          );
        }
        let mut grass_entries = Vec::with_capacity(mesh.grass.len());
        for i in 0 .. mesh.grass.len() {
          let chunk_id = mesh.grass.polygon_chunk_ids[i];
          let polygon_offset = mesh.grass.polygon_offsets[i];
          let chunk_idx = view.terrain_buffers.lookup_opengl_index(chunk_id).unwrap();
          let polygon_idx = chunk_idx.subindex(polygon_offset);
          grass_entries.push(
            view::grass_buffers::Entry {
              polygon_idx : polygon_idx.to_u32(),
              tex_id      : mesh.grass.tex_ids[i],
            }
          );
        }
        view.grass_buffers.push(
          &mut view.gl,
          grass_entries.as_ref(),
          mesh.grass.ids.as_ref(),
        );
      })
    },
    T::UnloadMesh(terrain_mesh::Ids { chunk_ids, grass_ids }) => {
      // Removing grass needs to happen before the calls to [update_polygon_index], or we will remove the wrong things.
      for id in grass_ids {
        view.grass_buffers.swap_remove(&mut view.gl, id);
      }
      for chunk_id in chunk_ids {
        match view.terrain_buffers.swap_remove(&mut view.gl, chunk_id) {
          None => {},
          Some((idx, swapped_idx)) => {
            for i in index::all() {
              view.grass_buffers.update_polygon_index(
                &mut view.gl,
                swapped_idx.subindex(i),
                idx.subindex(i),
              );
            }
          }
        }
      }
    },
    T::Atomic(updates) => {
      for up in updates {
        apply_client_to_view(view, up);
      }
    },
  };
}
