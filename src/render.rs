use gl;
use state::App;
use stopwatch;
use stopwatch::*;

// TODO: make this parameter non-mut
pub fn render<'a>(app: &mut App<'a>) {
  time!(app.timers.deref(), "render", || {
    app.gl.clear_buffer();

    app.color_shader.borrow_mut().set_camera(&mut app.gl, &app.player.camera);

    app.gl.use_shader(app.color_shader.borrow().deref(), |_| {
      // debug stuff
      app.line_of_sight.draw(&app.gl);

      if app.render_octree {
        app.octree_buffers.draw(&app.gl);
      }
    });

    app.texture_shader.borrow_mut().set_camera(&mut app.gl, &app.player.camera);

    // draw the world
    if app.render_outlines {
      gl::PolygonMode(gl::FRONT_AND_BACK, gl::LINE);
      gl::Disable(gl::CULL_FACE);
      app.gl.use_shader(app.texture_shader.borrow().deref(), |gl| {
        app.terrain_buffers.draw(gl);
      });
      app.gl.use_shader(app.color_shader.borrow().deref(), |gl| {
        app.mob_buffers.draw(gl);
      });
      gl::PolygonMode(gl::FRONT_AND_BACK, gl::FILL);
      gl::Enable(gl::CULL_FACE);
    } else {
      app.gl.use_shader(app.texture_shader.borrow().deref(), |gl| {
        app.terrain_buffers.draw(gl);
      });
      app.gl.use_shader(app.color_shader.borrow().deref(), |gl| {
        app.mob_buffers.draw(gl);
      });
    }

    // draw the hud
    app.gl.use_shader(app.hud_color_shader.borrow().deref(), |gl| {
      app.hud_triangles.draw(gl);
    });

    match gl::GetError() {
      gl::NO_ERROR => {},
      err => fail!("OpenGL error 0x{:x}", err),
    }

    // draw hud textures
    app.gl.use_shader(app.hud_texture_shader.borrow().deref(), |gl| {
      gl::ActiveTexture(app.misc_texture_unit.gl_id());
      for (i, tex) in app.text_textures.iter().enumerate() {
        tex.bind_2d(gl);
        app.text_triangles.draw_slice(gl, i * 2, 2);
      }
    });

    gl::Finish();
  })
}
