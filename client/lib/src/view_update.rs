//! Define the updates passed from the client to the view.

use cgmath::Point3;
use stopwatch;

use common::color::Color3;
use common::entity_id;

use block_position;
use light;
use light::{set_sun, set_ambient_light};
use lod;
use mob_buffers::VERTICES_PER_MOB;
use player_buffers::VERTICES_PER_PLAYER;
use terrain_mesh;
use vertex::ColoredVertex;
use view;

/// Messages from the client to the view.
pub enum ClientToView {
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

  /// Add a terrain block to the view.
  AddBlock(block_position::T, terrain_mesh::T, lod::T),
  /// Remove a terrain entity.
  RemoveTerrain(entity_id::T),
  /// Remove a grass billboard.
  RemoveGrass(entity_id::T),
  /// Treat a series of updates as an atomic operation.
  Atomic(Vec<ClientToView>),
}

unsafe impl Send for ClientToView {}

#[allow(missing_docs)]
pub fn apply_client_to_view(view: &mut view::T, up: ClientToView) {
  match up {
    ClientToView::MoveCamera(position) => {
      view.camera.translate_to(position);
    },
    ClientToView::UpdateMob(id, triangles) => {
      view.mob_buffers.insert(&mut view.gl, id, &triangles);
    },
    ClientToView::UpdatePlayer(id, triangles) => {
      view.player_buffers.insert(&mut view.gl, id, &triangles);
    },
    ClientToView::SetSun(sun) => {
      set_sun(
        &mut view.shaders.terrain_shader.shader,
        &mut view.gl,
        &sun,
      );
      view.sun = sun;
    },
    ClientToView::SetAmbientLight(color) => {
      set_ambient_light(
        &mut view.shaders.terrain_shader.shader,
        &mut view.gl,
        color,
      );
    },
    ClientToView::AddBlock(_, block, _) => {
      stopwatch::time("add_block", || {
        view.terrain_buffers.push(
          &mut view.gl,
          block.vertex_coordinates.as_ref(),
          block.normals.as_ref(),
          block.ids.as_ref(),
          block.materials.as_ref(),
        );
        view.grass_buffers.push(
          &mut view.gl,
          block.grass_vertices.as_ref(),
          block.grass_ids.as_ref(),
        );
      })
    },
    ClientToView::RemoveTerrain(id) => {
      view.terrain_buffers.swap_remove(&mut view.gl, id);
    },
    ClientToView::RemoveGrass(id) => {
      view.grass_buffers.swap_remove(&mut view.gl, id);
    },
    ClientToView::Atomic(updates) => {
      for up in updates.into_iter() {
        apply_client_to_view(view, up);
      }
    },
  };
}
