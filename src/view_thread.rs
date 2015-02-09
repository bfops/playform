use camera;
use color::{Color3, Color4};
use gl::types::*;
use nalgebra::{Vec2, Pnt3};
use light::{Light, set_point_light, set_ambient_light};
use std::iter::repeat;
use terrain::terrain_block::{BlockPosition, TerrainBlock};
use vertex::{ColoredVertex, TextureVertex};
use view::View;
use world::EntityId;

// TODO: Make the view updates resemble updates that would come from a server.
#[derive(Clone)]
pub enum ViewUpdate {
  PushHudTriangles(Vec<ColoredVertex>),
  PushTextTriangles(Vec<TextureVertex>),
  PushText((Color4<u8>, String)),
  PushMob((EntityId, Vec<ColoredVertex>)),
  UpdateMob((EntityId, Vec<ColoredVertex>)),
  SetPointLight(Light),
  SetAmbientLight(Color3<GLfloat>),
  SetBackgroundColor(Color3<GLfloat>),
  MoveCamera(Pnt3<GLfloat>),
  RotateCamera(Vec2<GLfloat>),
  RemoveTerrain(EntityId),
  PushBlock((BlockPosition, TerrainBlock, u32)),
  FreeBlock((BlockPosition, u32)),
}

impl ViewUpdate {
  pub fn apply(self, view: &mut View) {
    match self {
      ViewUpdate::PushHudTriangles(triangles) => {
        view.hud_triangles.bind(&mut view.gl);
        view.hud_triangles.push(&mut view.gl, triangles.as_slice());
      },
      ViewUpdate::PushTextTriangles(triangles) => {
        view.text_triangles.bind(&mut view.gl);
        view.text_triangles.push(&mut view.gl, triangles.as_slice());
      },
      ViewUpdate::PushText((color, s)) => {
        let tex = view.fontloader.sans.render(
          &view.gl,
          s.as_slice(),
          color,
        );
        view.text_textures.push(tex);
      },
      ViewUpdate::PushMob((id, triangles)) => {
        view.mob_buffers.push(&mut view.gl, id, triangles.as_slice());
      },
      ViewUpdate::UpdateMob((id, triangles)) => {
        view.mob_buffers.update(&mut view.gl, id, triangles.as_slice());
      },
      ViewUpdate::SetPointLight(light) => {
        set_point_light(
          &mut view.shaders.terrain_shader.shader,
          &mut view.gl,
          &light,
        );
      },
      ViewUpdate::SetAmbientLight(color) => {
        set_ambient_light(
          &mut view.shaders.terrain_shader.shader,
          &mut view.gl,
          color,
        );
      },
      ViewUpdate::SetBackgroundColor(color) => {
        view.gl.set_background_color(color.r, color.g, color.b, 1.0);
      },
      ViewUpdate::MoveCamera(pos) => {
        view.camera.translation = camera::translation(-pos.to_vec());
      },
      ViewUpdate::RotateCamera(rot) => {
        view.rotate_lateral(rot.x);
        view.rotate_vertical(rot.y);
      },
      ViewUpdate::RemoveTerrain(id) => {
        view.terrain_buffers.swap_remove(&mut view.gl, id);
      },
      ViewUpdate::PushBlock((block_position, block, lod)) => {
        if !block.ids.is_empty() {
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
        }
      },
      ViewUpdate::FreeBlock((block_position, lod)) => {
        view.terrain_buffers.free_block_data(lod, &block_position);
      },
    };
  }
}

unsafe impl Send for ViewUpdate {}
