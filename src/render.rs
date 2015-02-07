use camera::set_camera;
use gl;
use renderer::Renderer;
use state::App;
use stopwatch::TimerSet;

pub fn render(
  timers: &TimerSet,
  app: &App,
  renderer: &mut Renderer,
) {
  timers.time("render", || {
    &mut renderer.gl.clear_buffer();

    set_camera(&mut renderer.shaders.mob_shader.shader, &mut renderer.gl, &app.player.camera);

    renderer.shaders.mob_shader.shader.use_shader(&mut renderer.gl);

    set_camera(&mut renderer.shaders.terrain_shader.shader, &mut renderer.gl, &app.player.camera);

    // draw the world
    if app.render_outlines {
      unsafe {
        gl::PolygonMode(gl::FRONT_AND_BACK, gl::LINE);
        gl::Disable(gl::CULL_FACE);
      }

      renderer.shaders.terrain_shader.shader.use_shader(&mut renderer.gl);
      renderer.terrain_buffers.draw(&mut renderer.gl);

      renderer.shaders.mob_shader.shader.use_shader(&mut renderer.gl);
      renderer.mob_buffers.draw(&mut renderer.gl);

      unsafe {
        gl::PolygonMode(gl::FRONT_AND_BACK, gl::FILL);
        gl::Enable(gl::CULL_FACE);
      }
    } else {
      renderer.shaders.terrain_shader.shader.use_shader(&mut renderer.gl);
      renderer.terrain_buffers.draw(&mut renderer.gl);

      renderer.shaders.mob_shader.shader.use_shader(&mut renderer.gl);
      renderer.mob_buffers.draw(&mut renderer.gl);
    }

    // draw the hud
    renderer.shaders.hud_color_shader.shader.use_shader(&mut renderer.gl);
    renderer.hud_triangles.bind(&mut renderer.gl);
    renderer.hud_triangles.draw(&mut renderer.gl);

    // draw hud textures
    renderer.shaders.hud_texture_shader.shader.use_shader(&mut renderer.gl);
    unsafe {
      gl::ActiveTexture(renderer.misc_texture_unit.gl_id());
    }

    renderer.text_triangles.bind(&mut renderer.gl);
    for (i, tex) in renderer.text_textures.iter().enumerate() {
      unsafe {
        gl::BindTexture(gl::TEXTURE_2D, tex.handle.gl_id);
      }
      renderer.text_triangles.draw_slice(&mut renderer.gl, i * 6, 6);
    }
  })
}
