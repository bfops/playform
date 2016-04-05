use common::interval_timer::IntervalTimer;
use time;
use std;

pub struct Sun {
  // The sun as portions of a 65536-degree circle.
  pub position: u16,
  pub timer: IntervalTimer,
  pub print_timer: IntervalTimer,
}

impl Sun {
  pub fn new(tick_ns: u64) -> Sun {
    Sun {
      position: 0,
      timer: IntervalTimer::new(tick_ns, time::precise_time_ns()),
      print_timer: IntervalTimer::new(2e9 as u64, time::precise_time_ns()),
    }
  }

  pub fn update(&mut self) -> Option<f32> {
    let ticks = self.timer.update(time::precise_time_ns());

    if ticks == 0 {
      return None;
    }

    self.position = (std::num::Wrapping(self.position) + std::num::Wrapping(ticks as u16)).0;

    // Fraction completed of a full cycle.
    let fraction = (self.position as f32) / 65536.0;
    // Longer day, shorter night.
    let fraction = fraction * fraction;

    if self.print_timer.update(time::precise_time_ns()) > 0 {
      debug!("Sun is at {:.1}%.", fraction * 100.0);
    }

    Some(fraction)
  }
}
