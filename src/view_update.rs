use color::Color3;
use nalgebra::Pnt3;
use light::{Light, set_point_light, set_ambient_light};
use std::iter::repeat;
use terrain::terrain_block::{BlockPosition, TerrainBlock};
use vertex::ColoredVertex;
use view::View;
use server::EntityId;

#[derive(Clone)]
pub enum ClientToView {
  MoveCamera(Pnt3<f32>),

  AddMob(EntityId, Vec<ColoredVertex>),
  UpdateMob(EntityId, Vec<ColoredVertex>),

  SetPointLight(Light),
  SetAmbientLight(Color3<f32>),
  SetClearColor(Color3<f32>),

  AddBlock(BlockPosition, TerrainBlock, u32),
  RemoveTerrain(EntityId),
  RemoveBlockData(BlockPosition, u32),
}

impl ClientToView {
  pub fn apply(self, view: &mut View) {
    match self {
      ClientToView::MoveCamera(position) => {
        view.camera.translate_to(position);
      },
      ClientToView::AddMob(id, triangles) => {
        view.mob_buffers.push(&mut view.gl, id, triangles.as_slice());
      },
      ClientToView::UpdateMob(id, triangles) => {
        view.mob_buffers.update(&mut view.gl, id, triangles.as_slice());
      },
      ClientToView::SetPointLight(light) => {
        set_point_light(
          &mut view.shaders.terrain_shader.shader,
          &mut view.gl,
          &light
        );
      },
      ClientToView::SetAmbientLight(color) => {
        set_ambient_light(
          &mut view.shaders.terrain_shader.shader,
          &mut view.gl,
          color,
        );
      },
      ClientToView::SetClearColor(color) => {
        view.gl.set_background_color(color.r, color.g, color.b, 1.0);
      },
      ClientToView::AddBlock(block_position, block, lod) => {
        let block_index =
          view.terrain_buffers.push_block_data(
            &mut view.gl,
            block_position,
            block.pixels.as_slice(),
            lod,
          );

        let block_indices: Vec<_> =
          repeat(block_index).take(block.ids.len()).collect();

        view.terrain_buffers.push(
          &mut view.gl,
          block.vertex_coordinates.as_slice(),
          block.normals.as_slice(),
          block.coords.as_slice(),
          block_indices.as_slice(),
          block.ids.as_slice(),
        );
      },
      ClientToView::RemoveTerrain(id) => {
        view.terrain_buffers.swap_remove(&mut view.gl, id);
      },
      ClientToView::RemoveBlockData(block_position, lod) => {
        view.terrain_buffers.free_block_data(lod, &block_position);
      },
    };
  }
}

unsafe impl Send for ClientToView {}
