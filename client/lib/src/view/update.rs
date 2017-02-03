//! Define the updates passed from the client to the view.

use cgmath::Point3;
use stopwatch;

use common::entity_id;

use terrain_mesh;
use vertex::ColoredVertex;
use view;

use super::chunk_stats;
use super::light;
use super::mob_buffers::VERTICES_PER_MOB;
use super::player_buffers::VERTICES_PER_PLAYER;
use super::terrain_buffers;

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
  LoadMesh (Box<terrain_mesh::T>),
  /// Remove a terrain entity.
  UnloadMesh(terrain_mesh::Ids),
  /// Treat a series of updates as an atomic operation.
  Atomic(Vec<T>),
}

unsafe impl Send for T {}

pub use self::T::*;

#[allow(missing_docs)]
pub fn apply_client_to_view(view: &mut view::T, chunk_stats: &mut chunk_stats::T, up: T) {
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
        let terrain_mesh::T { vertex_coordinates, normals, ids, materials, grass, .. } = mesh;
        chunk_stats.add(vertex_coordinates.len());
        view.terrain_buffers.push(
          &mut view.gl,
          vertex_coordinates,
          normals,
          ids.chunk_id,
          materials,
        );
        let mut grass_entries = Vec::with_capacity(grass.len());
        let mut polygon_indices = Vec::with_capacity(grass.len());
        for (i, g) in grass.iter().enumerate() {
          let g: &terrain_mesh::Grass = g;
          let polygon_idx = view.terrain_buffers.lookup_opengl_index(ids.chunk_id).unwrap() * terrain_buffers::VRAM_CHUNK_LENGTH as u32 + i as u32;
          grass_entries.push(
            view::grass_buffers::Entry {
              polygon_idx : polygon_idx,
              tex_id      : g.tex_id,
            }
          );
          polygon_indices.push(polygon_idx);
        }
        view.grass_buffers.push(
          &mut view.gl,
          grass_entries.as_ref(),
          polygon_indices.as_ref(),
          ids.grass_ids.as_ref(),
        );
      })
    },
    T::UnloadMesh(terrain_mesh::Ids { chunk_id, grass_ids }) => {
      match view.terrain_buffers.swap_remove(&mut view.gl, chunk_id) {
        None => {},
        Some((idx, swapped_idx)) => {
          let swapped_base = swapped_idx * terrain_buffers::VRAM_CHUNK_LENGTH;
          let base = idx * terrain_buffers::VRAM_CHUNK_LENGTH;
          for i in 0..terrain_buffers::VRAM_CHUNK_LENGTH {
            view.grass_buffers.update_polygon_index(
              &mut view.gl,
              (swapped_base + i) as u32,
              (base + i) as u32,
            );
          }
        }
      }
      for id in grass_ids {
        view.grass_buffers.swap_remove(&mut view.gl, id);
      }
    },
    T::Atomic(updates) => {
      for up in updates {
        apply_client_to_view(view, chunk_stats, up);
      }
    },
  };
}
