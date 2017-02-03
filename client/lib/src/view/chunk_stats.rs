//! record stats about polygon counts in chunks

use std;
use std::io::Write;

const MAX_POLYGON_COUNT: usize = 1 << 13;

#[allow(missing_docs)]
pub struct T {
  chunk_counts_by_polygon_count: [u32; MAX_POLYGON_COUNT]
}

#[allow(missing_docs)]
pub fn new() -> T {
  T {
    chunk_counts_by_polygon_count: [0; MAX_POLYGON_COUNT]
  }
}

impl T {
  /// record a chunk
  pub fn add(&mut self, polygon_count: u32) {
    let polygon_count = polygon_count as usize;
    if polygon_count >= MAX_POLYGON_COUNT {
      //warn!("Chunk too big to record in stats: {}" polygon_count);
      return
    }

    self.chunk_counts_by_polygon_count[polygon_count] += 1;
  }

  /// print all the stats into a file
  pub fn output_to(&self, file: &str) {
    let mut file = std::fs::File::create(file).unwrap();

    file.write_all(b"vram_chunk_loads = [").unwrap();
    for (i, record) in self.chunk_counts_by_polygon_count.iter().enumerate() {
      if i > 0 {
        file.write_all(b", ").unwrap();
      }
      file.write_fmt(format_args!("{}", record)).unwrap();
    }
    file.write_all(b"];\n").unwrap();
  }
}
