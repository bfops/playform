use common::*;
use gl;
use interval_timer::IntervalTimer;
use process_event::process_event;
use render::render;
use sdl2;
use sdl2::event::Event;
use state::App;
use std::mem;
use std::time::duration::Duration;
use std::io::timer;
use stopwatch::TimerSet;
use time;
use update::update;
use yaglw::gl_context::{GLContext, GLContextExistence};

pub const FRAMES_PER_SECOND: u64 = 30;
pub const UPDATES_PER_SECOND: u64 = 30;

pub fn main() {
  debug!("starting");

  with_sdl2_gl_loaded(|window, gl, gl_context| {
    let timers = TimerSet::new();
    let mut app = App::new(gl, gl_context, &timers);

    let mut render_timer;
    let mut update_timer;
    {
      let now = time::precise_time_ns();
      let nanoseconds_per_second = 1000000000;
      render_timer = IntervalTimer::new(nanoseconds_per_second / FRAMES_PER_SECOND, now);
      update_timer = IntervalTimer::new(nanoseconds_per_second / UPDATES_PER_SECOND, now);
    }

    loop {
      let updates = update_timer.update(time::precise_time_ns());
      if updates > 0 {
        update(&mut app);
      }

      let renders = render_timer.update(time::precise_time_ns());
      if renders > 0 {
        render(&mut app);
        // swap buffers
        window.gl_swap_window();
      }

      loop {
        match sdl2::event::poll_event() {
          Event::None => {
            break;
          },
          Event::Quit(_) => {
            return;
          }
          Event::AppTerminating(_) => {
            return;
          }
          event => {
            process_event(&mut app, window, event);
          },
        }
      }
      timer::sleep(Duration::microseconds(10));
    }
  });

  debug!("finished");
}

#[allow(unused_variables)]
fn with_sdl2_gl_loaded(act: |&mut sdl2::video::Window, &GLContextExistence, &mut GLContext|) {
  sdl2::init(sdl2::INIT_EVERYTHING);

  sdl2::video::gl_set_attribute(sdl2::video::GLAttr::GLContextMajorVersion, 3);
  sdl2::video::gl_set_attribute(sdl2::video::GLAttr::GLContextMinorVersion, 3);
  sdl2::video::gl_set_attribute(
    sdl2::video::GLAttr::GLContextProfileMask,
    sdl2::video::GLProfile::GLCoreProfile as int
  );

  let mut window = sdl2::video::Window::new(
    "playform",
    sdl2::video::WindowPos::PosCentered,
    sdl2::video::WindowPos::PosCentered,
    WINDOW_WIDTH as int,
    WINDOW_HEIGHT as int,
    sdl2::video::OPENGL,
  ).unwrap();

  // Send text input events.
  sdl2::keyboard::start_text_input();

  let sdl_gl_context = window.gl_create_context().unwrap();

  // Load the OpenGL function pointers.
  gl::load_with(|s| unsafe {
    mem::transmute(sdl2::video::gl_get_proc_address(s))
  });

  let (gl, mut gl_context) = unsafe {
    GLContext::new()
  };

  act(&mut window, &gl, &mut gl_context);
}
