//! Draw the view.

use camera::set_camera;
use gl;
use view;

#[allow(missing_docs)]
pub fn render(
  rndr: &mut view::T,
) {
  to_fbo(rndr);
  from_fbo(rndr);
}

fn to_fbo(rndr: &mut view::T) {
  rndr.deferred_fbo.bind(&mut rndr.gl);

  &mut rndr.gl.clear_buffer();

  set_camera(&mut rndr.shaders.terrain.shader, &mut rndr.gl, &rndr.camera);

  // draw the world
  rndr.shaders.terrain.shader.use_shader(&mut rndr.gl);
  rndr.terrain_buffers.draw(&mut rndr.gl);
}

fn from_fbo(rndr: &mut view::T) {
  unsafe {
    gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, 0);
  }

  &mut rndr.gl.clear_buffer();

  rndr.shaders.deferred.shader.use_shader(&mut rndr.gl);

  unsafe {
    gl::BindVertexArray(rndr.empty_vao.gl_id);
    gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
  }

  set_camera(&mut rndr.shaders.mob.shader, &mut rndr.gl, &rndr.camera);

  rndr.shaders.mob.shader.use_shader(&mut rndr.gl);
  rndr.mob_buffers.draw(&mut rndr.gl);
  rndr.player_buffers.draw(&mut rndr.gl);

  // draw the hud
  rndr.shaders.hud_color.shader.use_shader(&mut rndr.gl);
  rndr.hud_triangles.bind(&mut rndr.gl);
  rndr.hud_triangles.draw(&mut rndr.gl);

  // draw hud textures
  rndr.shaders.hud_texture.shader.use_shader(&mut rndr.gl);
  unsafe {
    gl::ActiveTexture(rndr.misc_texture_unit.gl_id());
  }

  rndr.text_triangles.bind(&mut rndr.gl);
  for (i, tex) in rndr.text_textures.iter().enumerate() {
    unsafe {
      gl::BindTexture(gl::TEXTURE_2D, tex.handle.gl_id);
    }
    rndr.text_triangles.draw_slice(&mut rndr.gl, i * 6, 6);
  }
}
