use common::*;
use current::Get;
use event::WindowSettings;
use interval_timer::IntervalTimer;
use process_input::process_input;
use render::render;
use sdl2_window::*;
use shader_version::opengl::OpenGL;
use state::App;
use stopwatch::TimerSet;
use time;
use update::update;
use window::{PollEvent, ShouldClose, SwapBuffers};
use yaglw::gl_context::GLContext;

pub const FRAMES_PER_SECOND: u64 = 30;
pub const UPDATES_PER_SECOND: u64 = 30;

pub fn main() {
  debug!("starting");

  let mut window = Sdl2Window::new(
    OpenGL::_3_3,
    WindowSettings {
      title: "playform".to_string(),
      size: [WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32],
      fullscreen: false,
      exit_on_esc: false,
      samples: 0,
    }
  );

  let (gl, mut gl_context) = unsafe {
    GLContext::new()
  };

  let timers = TimerSet::new();
  let mut app = App::new(&gl, &mut gl_context, &timers);

  let mut render_timer;
  let mut update_timer;
  {
    let now = time::precise_time_ns();
    let nanoseconds_per_second = 1000000000;
    render_timer = IntervalTimer::new(nanoseconds_per_second / FRAMES_PER_SECOND, now);
    update_timer = IntervalTimer::new(nanoseconds_per_second / UPDATES_PER_SECOND, now);
  }

  let mut should_close = false;
  while !should_close {
    let updates = update_timer.update(time::precise_time_ns());
    if updates > 0 {
      update(&mut app);
    }

    let renders = render_timer.update(time::precise_time_ns());
    if renders > 0 {
      render(&mut app);
      window.swap_buffers();
    }

    while let Some(input) = window.poll_event() {
      process_input(&mut app, &mut window, input);
    }

    let ShouldClose(b) = window.get();
    should_close = b;
  }

  debug!("finished!");
}
