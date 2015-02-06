use color::Color3;
use interval_timer::IntervalTimer;
use nalgebra::Vec3;
use std::cmp::partial_max;
use std::f32::consts::PI;
use std::num::Float;
use time;

pub struct Sun {
  // The sun as portions of a 65536-degree circle.
  pub position: u16,
  pub timer: IntervalTimer,
}

impl Sun {
  pub fn new(tick_ns: u64) -> Sun {
    Sun {
      position: 0,
      timer: IntervalTimer::new(tick_ns, time::precise_time_ns()),
    }
  }

  pub fn update(&mut self) -> (Vec3<f32>, Color3<f32>, f32) {
    let ticks = self.timer.update(time::precise_time_ns());
    self.position += ticks as u16;

    let radius = 1024.0;
    // Convert the sun angle to radians.
    let angle = (self.position as f32) * 2.0 * PI / 65536.0;
    let (s, c) = angle.sin_cos();
    let sun_position = Vec3::new(c, s, 0.0) * radius;

    let r = c.abs();
    let g = (s + 1.0) / 2.0;
    let b = (s * 0.75 + 0.25).abs();
    let sun_color = Color3::of_rgb(r, g, b);

    let ambient_light = partial_max(0.4, s / 2.0).unwrap();

    (sun_position, sun_color, ambient_light)
  }
}
