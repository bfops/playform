use gl::types::GLfloat;
use cgmath::vector;

#[deriving(Clone)]
pub struct BoundingBox {
  pub low_corner: vector::Vector3<GLfloat>,
  pub high_corner: vector::Vector3<GLfloat>,
}
