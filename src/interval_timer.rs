// to keep track of events that should happen at regular intervals
pub struct IntervalTimer {
  every: u64,
  next: u64,
}

impl IntervalTimer {
  pub fn new(every: u64, next: u64) -> IntervalTimer {
    IntervalTimer {
      every: every,
      next: next,
    }
  }

  #[inline]
  // Returns the number of intervals that have elapsed.
  pub fn update(&mut self, current: u64) -> u64 {
    if current < self.next {
      0
    } else {
      let r = 1 + (current - self.next) / self.every;
      self.next += self.every * r;
      r
    }
  }
}

#[test]
fn simple() {
  let mut timer = IntervalTimer::new(3, 3);
  let mut time = 2;
  assert_eq!(timer.update(time), 0);
  time += 2;
  assert_eq!(timer.update(time), 1);
  time += 2;
  assert_eq!(timer.update(time), 1);
  time += 6;
  assert_eq!(timer.update(time), 2);
  time += 3;
  assert_eq!(timer.update(time), 1);
  time += 2;
  assert_eq!(timer.update(time), 0);
}
