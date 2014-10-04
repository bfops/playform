pub struct Borrow<T> {
  #[allow(dead_code)]
  borrowed: T,
}

impl<T> Borrow<T> {
  pub fn borrow(borrowed: T) -> Borrow<T> {
    Borrow {
      borrowed: borrowed,
    }
  }
}
