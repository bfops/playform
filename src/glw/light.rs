use gl::types::GLfloat;
use nalgebra::na::Vec3;

pub struct Light {
  pub position: Vec3<GLfloat>,
  pub intensity: Vec3<GLfloat>,
}
