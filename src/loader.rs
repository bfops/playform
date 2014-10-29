use std::collections::RingBuf;

#[deriving(Clone)]
pub enum Operation<L, U> {
  Load(L),
  Unload(U),
}

pub type Loader<L, U> = RingBuf<Operation<L, U>>;
