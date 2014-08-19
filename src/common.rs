pub static WINDOW_WIDTH:  uint = 800;
pub static WINDOW_HEIGHT: uint = 600;

pub static TRIANGLES_PER_BOX: uint = 12;
pub static VERTICES_PER_TRIANGLE: uint = 3;
pub static TRIANGLE_VERTICES_PER_BOX: uint = TRIANGLES_PER_BOX * VERTICES_PER_TRIANGLE;

pub static VERTICES_PER_LINE: uint = 2;
pub static LINES_PER_BOX: uint = 12;
pub static LINE_VERTICES_PER_BOX: uint = LINES_PER_BOX * VERTICES_PER_LINE;

pub fn partial_min_by<A: Copy, T: Iterator<A>, B: PartialOrd>(t: T, f: |A| -> B) -> Option<A> {
  let mut t = t;
  let (mut min_a, mut min_b) = {
    match t.next() {
      None => return None,
      Some(a) => (a, f(a)),
    }
  };
  for a in t {
    let b = f(a);
    assert!(b != min_b);
    if b < min_b {
      min_a = a;
      min_b = b;
    }
  }

  Some(min_a)
}
