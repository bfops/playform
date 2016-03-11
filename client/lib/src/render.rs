//! Draw the view.

use camera::set_camera;
use gl;
use std;
use view;

fn draw_backdrop(
  rndr: &mut view::T,
) {
  rndr.shaders.clouds.shader.use_shader(&mut rndr.gl);

  unsafe {
    gl::BindVertexArray(rndr.empty_gl_array.gl_id);
  }

  let sun_uniform = rndr.shaders.clouds.shader.get_uniform_location("sun_color");
  unsafe {
    let ptr = std::mem::transmute(&rndr.sun.intensity);
    gl::Uniform3fv(sun_uniform, 1, ptr);

    gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);

    gl::Clear(gl::DEPTH_BUFFER_BIT);
  }
}

#[allow(missing_docs)]
pub fn render(
  rndr: &mut view::T,
) {
  rndr.gl.clear_buffer();

  draw_backdrop(rndr);

  set_camera(&mut rndr.shaders.mob_shader.shader, &mut rndr.gl, &rndr.camera);

  rndr.shaders.mob_shader.shader.use_shader(&mut rndr.gl);

  set_camera(&mut rndr.shaders.terrain_shader.shader, &mut rndr.gl, &rndr.camera);

  // draw the world
  rndr.shaders.terrain_shader.shader.use_shader(&mut rndr.gl);
  rndr.terrain_buffers.draw(&mut rndr.gl);

  rndr.shaders.mob_shader.shader.use_shader(&mut rndr.gl);
  rndr.mob_buffers.draw(&mut rndr.gl);
  rndr.player_buffers.draw(&mut rndr.gl);

  if rndr.show_hud {
    rndr.shaders.hud_color_shader.shader.use_shader(&mut rndr.gl);
    rndr.hud_triangles.bind(&mut rndr.gl);
    rndr.hud_triangles.draw(&mut rndr.gl);

    // draw hud textures
    rndr.shaders.hud_texture_shader.shader.use_shader(&mut rndr.gl);
    unsafe {
      gl::ActiveTexture(rndr.misc_texture_unit.gl_id());
    }
  }
}
