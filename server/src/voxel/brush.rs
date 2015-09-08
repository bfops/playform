use cgmath::Aabb3;

pub type Bounds = Aabb3<i32>;

#[derive(Debug, Clone)]
pub struct T<Mosaic> {
  pub bounds: Bounds,
  pub mosaic: Mosaic,
}

unsafe impl<Mosaic> Send for T<Mosaic> {}
