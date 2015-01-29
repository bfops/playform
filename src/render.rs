use camera::set_camera;
use gl;
use state::App;
use stopwatch::TimerSet;
use yaglw::gl_context::GLContext;

pub fn render(
  timers: &TimerSet,
  // TODO: make this parameter non-mut by factoring out the shaders.
  app: &mut App,
  gl_context: &mut GLContext,
) {
  timers.time("render", || {
    gl_context.clear_buffer();

    set_camera(&mut app.mob_shader.shader, gl_context, &app.player.camera);

    app.mob_shader.shader.use_shader(gl_context);

    // debug stuff
    app.line_of_sight.bind(gl_context);
    app.line_of_sight.draw(gl_context);

    set_camera(&mut app.terrain_shader.shader, gl_context, &app.player.camera);

    // draw the world
    if app.render_outlines {
      unsafe {
        gl::PolygonMode(gl::FRONT_AND_BACK, gl::LINE);
        gl::Disable(gl::CULL_FACE);
      }

      app.terrain_shader.shader.use_shader(gl_context);
      app.terrain_game_loader.terrain_vram_buffers.draw(gl_context);
      app.mob_shader.shader.use_shader(gl_context);
      app.mob_buffers.draw(gl_context);

      unsafe {
        gl::PolygonMode(gl::FRONT_AND_BACK, gl::FILL);
        gl::Enable(gl::CULL_FACE);
      }
    } else {
      app.terrain_shader.shader.use_shader(gl_context);
      app.terrain_game_loader.terrain_vram_buffers.draw(gl_context);
      app.mob_shader.shader.use_shader(gl_context);
      app.mob_buffers.draw(gl_context);
    }

    // draw the hud
    app.hud_color_shader.shader.use_shader(gl_context);
    app.hud_triangles.bind(gl_context);
    app.hud_triangles.draw(gl_context);

    // draw hud textures
    app.hud_texture_shader.shader.use_shader(gl_context);
    unsafe {
      gl::ActiveTexture(app.misc_texture_unit.gl_id());
    }

    app.text_triangles.bind(gl_context);
    for (i, tex) in app.text_textures.iter().enumerate() {
      unsafe {
        gl::BindTexture(gl::TEXTURE_2D, tex.handle.gl_id);
      }
      app.text_triangles.draw_slice(gl_context, i * 6, 6);
    }
  })
}
