use camera::set_camera;
use gl;
use state::App;

// TODO: make this parameter non-mut
pub fn render<'a>(app: &mut App<'a>) {
  app.timers.time("render", || {
    app.gl_context.clear_buffer();

    set_camera(app.color_shader.borrow_mut().deref_mut(), app.gl_context, &app.player.camera);

    app.color_shader.borrow().use_shader(app.gl_context);

    // debug stuff
    app.line_of_sight.bind(app.gl_context);
    app.line_of_sight.draw(app.gl_context);

    set_camera(app.texture_shader.borrow_mut().deref_mut(), app.gl_context, &app.player.camera);

    // draw the world
    if app.render_outlines {
      unsafe {
        gl::PolygonMode(gl::FRONT_AND_BACK, gl::LINE);
        gl::Disable(gl::CULL_FACE);
      }

      app.texture_shader.borrow().use_shader(app.gl_context);
      app.terrain_buffers.draw(app.gl_context);
      app.color_shader.borrow().use_shader(app.gl_context);
      app.mob_buffers.draw(app.gl_context);

      unsafe {
        gl::PolygonMode(gl::FRONT_AND_BACK, gl::FILL);
        gl::Enable(gl::CULL_FACE);
      }
    } else {
      app.texture_shader.borrow().use_shader(app.gl_context);
      app.terrain_buffers.draw(app.gl_context);
      app.color_shader.borrow().use_shader(app.gl_context);
      app.mob_buffers.draw(app.gl_context);
    }

    // draw the hud
    app.hud_color_shader.borrow().use_shader(app.gl_context);
    app.hud_triangles.bind(app.gl_context);
    app.hud_triangles.draw(app.gl_context);

    // draw hud textures
    app.hud_texture_shader.borrow().use_shader(app.gl_context);
    unsafe {
      gl::ActiveTexture(app.misc_texture_unit.gl_id());
    }

    app.text_triangles.bind(app.gl_context);
    for (i, tex) in app.text_textures.iter().enumerate() {
      unsafe {
        gl::BindTexture(gl::TEXTURE_2D, tex.handle.gl_id);
      }
      app.text_triangles.draw_slice(app.gl_context, i * 6, 6);
    }
  })
}
