//! Draw the view.

use camera::set_camera;
use gl;
use view::View;

#[allow(missing_docs)]
pub fn render(
  rndr: &mut View,
) {
  &mut rndr.gl.clear_buffer();

  set_camera(&mut rndr.shaders.mob_shader.shader, &mut rndr.gl, &rndr.camera);

  rndr.shaders.mob_shader.shader.use_shader(&mut rndr.gl);

  set_camera(&mut rndr.shaders.terrain_shader.shader, &mut rndr.gl, &rndr.camera);

  // draw the world
  rndr.shaders.terrain_shader.shader.use_shader(&mut rndr.gl);
  rndr.terrain_buffers.draw(&mut rndr.gl);

  rndr.shaders.mob_shader.shader.use_shader(&mut rndr.gl);
  rndr.mob_buffers.draw(&mut rndr.gl);
  rndr.player_buffers.draw(&mut rndr.gl);

  // draw the hud
  rndr.shaders.hud_color_shader.shader.use_shader(&mut rndr.gl);
  rndr.hud_triangles.bind(&mut rndr.gl);
  rndr.hud_triangles.draw(&mut rndr.gl);

  // draw hud textures
  rndr.shaders.hud_texture_shader.shader.use_shader(&mut rndr.gl);
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
