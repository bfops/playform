//! Draw the view.

use camera::{Camera, set_camera};
use cgmath;
use light::{set_sun, set_ambient_light};
use gl;
use time;
use view;
use yaglw;

fn set_eye_position(shader: &mut yaglw::shader::Shader, camera: &Camera) {
  unsafe {
    let uniform = shader.get_uniform_location("eye_position");
    let ptr = &camera.position as *const _ as *const _;
    gl::Uniform3fv(uniform, 1, ptr);
  }
}

fn draw_backdrop(
  rndr: &mut view::T,
) {
  rndr.shaders.sky.shader.use_shader(&mut rndr.gl);

  unsafe {
    gl::BindVertexArray(rndr.empty_gl_array.gl_id);
  }

  set_sun(&mut rndr.shaders.sky.shader, &mut rndr.gl, &rndr.sun);
  set_eye_position(&mut rndr.shaders.sky.shader, &rndr.camera);

  unsafe {
    let time_ms_uniform = rndr.shaders.sky.shader.get_uniform_location("time_ms");
    gl::Uniform1f(time_ms_uniform, (time::precise_time_ns() / 1_000_000) as f32);

    let projection_uniform = rndr.shaders.sky.shader.get_uniform_location("projection_matrix");
    let projection_matrix = rndr.camera.projection_matrix();
    let ptr = &projection_matrix as *const _ as *const _;
    gl::UniformMatrix4fv(projection_uniform, 1, 0, ptr);

    let window_size_uniform = rndr.shaders.sky.shader.get_uniform_location("window_size");
    let window_size = cgmath::Vector2::new(rndr.window_size.x as f32, rndr.window_size.y as f32);
    let ptr = &window_size as *const _ as *const _;
    gl::Uniform2fv(window_size_uniform, 1, ptr);

    gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);

    gl::Clear(gl::DEPTH_BUFFER_BIT);
  }
}

fn draw_grass_billboards(
  rndr: &mut view::T,
) {
  rndr.shaders.grass_billboard.shader.use_shader(&mut rndr.gl);
  set_camera(&mut rndr.shaders.grass_billboard.shader, &mut rndr.gl, &rndr.camera);
  set_eye_position(&mut rndr.shaders.grass_billboard.shader, &rndr.camera);
  set_sun(&mut rndr.shaders.grass_billboard.shader, &mut rndr.gl, &rndr.sun);
  set_ambient_light(&mut rndr.shaders.grass_billboard.shader, &mut rndr.gl, &rndr.sun);
  let alpha_threshold_uniform =
    rndr.shaders.grass_billboard.shader.get_uniform_location("alpha_threshold");
  unsafe {
    gl::Disable(gl::CULL_FACE);
    gl::Uniform1f(alpha_threshold_uniform, 0.5);
    gl::ActiveTexture(rndr.misc_texture_unit.gl_id());
    gl::BindTexture(gl::TEXTURE_2D, rndr.grass_texture.handle.gl_id);
  }
  rndr.grass_buffers.draw(&mut rndr.gl);
}

#[allow(missing_docs)]
pub fn render(
  rndr: &mut view::T,
) {
  rndr.gl.clear_buffer();

  draw_backdrop(rndr);

  unsafe {
    gl::Enable(gl::CULL_FACE);
  }

  // draw the world
  rndr.shaders.terrain_shader.shader.use_shader(&mut rndr.gl);
  set_sun(&mut rndr.shaders.terrain_shader.shader, &mut rndr.gl, &rndr.sun);
  set_ambient_light(&mut rndr.shaders.terrain_shader.shader, &mut rndr.gl, &rndr.sun);
  set_camera(&mut rndr.shaders.terrain_shader.shader, &mut rndr.gl, &rndr.camera);
  set_eye_position(&mut rndr.shaders.terrain_shader.shader, &rndr.camera);
  rndr.terrain_buffers.draw(&mut rndr.gl);

  rndr.shaders.mob_shader.shader.use_shader(&mut rndr.gl);
  set_camera(&mut rndr.shaders.mob_shader.shader, &mut rndr.gl, &rndr.camera);
  rndr.mob_buffers.draw(&mut rndr.gl);
  rndr.player_buffers.draw(&mut rndr.gl);

  draw_grass_billboards(rndr);

  if rndr.show_hud {
    rndr.shaders.hud_color_shader.shader.use_shader(&mut rndr.gl);
    rndr.hud_triangles.bind(&mut rndr.gl);
    rndr.hud_triangles.draw(&mut rndr.gl);
  }
}
