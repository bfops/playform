use sdl2::event::Event;
use sdl2::video;
use sdl2_sys;
use std;
use stopwatch;
use time;

use client;
use hud::make_hud;
use process_event::process_event;
use view;
use view::update;

const MAX_POLYGON_COUNT: u32 = 1 << 13;

pub struct T {
  chunk_counts_by_polygon_count: [u32; MAX_POLYGON_COUNT]
}

pub fn new() -> T {
  T {
    buckets: [0; MAX_POLYGON_COUNT]
  }
}

impl T {
  pub fn add(&mut self, polygon_count: u32) {
    if polygon_count >= MAX_POLYGON_COUNT {
      warn!("Chunk too big to record in stats: {}" polygon_count);
      return
    }

    self.buckets[polygon_count] += 1;
  }

  pub fn output_to(&self, file: string) {
    let mut file = std::fs::File::create("vram_chunk_loads.out").unwrap();

    file.write_all(b"vram_chunk_loads = [").unwrap();
    for (i, record) in self.buckets.iter().enumerate() {
      if i > 0 {
        file.write_all(b", ").unwrap();
      }
      file.write_fmt(format_args!("{}", record));
    }
    file.write_all(b"];\n").unwrap();
  }
}
