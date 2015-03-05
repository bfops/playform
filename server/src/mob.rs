use cgmath::{Point3, Vector3};
use cgmath::Aabb3;

use common::color::Color4;
use common::entity::EntityId;
use common::lod::OwnerId;
use common::surroundings_loader::SurroundingsLoader;
use common::vertex::ColoredVertex;

use server::Server;

pub const TRIANGLES_PER_BOX: u32 = 12;
pub const VERTICES_PER_TRIANGLE: u32 = 3;
pub const TRIANGLE_VERTICES_PER_BOX: u32 = TRIANGLES_PER_BOX * VERTICES_PER_TRIANGLE;

pub type Behavior = fn(&Server, &mut Mob);

pub struct Mob {
  pub position: Point3<f32>,
  pub speed: Vector3<f32>,
  pub behavior: Behavior,

  pub entity_id: EntityId,
  pub owner_id: OwnerId,
  pub surroundings_loader: SurroundingsLoader,
}

impl Mob {
  pub fn to_triangles(
    bounds: &Aabb3<f32>,
    c: &Color4<f32>,
  ) -> [ColoredVertex; TRIANGLE_VERTICES_PER_BOX as usize] {
    let (x1, y1, z1) = (bounds.min.x, bounds.min.y, bounds.min.z);
    let (x2, y2, z2) = (bounds.max.x, bounds.max.y, bounds.max.z);

    let vtx = |x, y, z| {
      ColoredVertex {
        position: Point3::new(x, y, z),
        color: c.clone(),
      }
    };

    // Remember: x increases to the right, y increases up, and z becomes more
    // negative as depth from the viewer increases.
    [
      // front
      vtx(x1, y1, z2), vtx(x2, y2, z2), vtx(x1, y2, z2),
      vtx(x1, y1, z2), vtx(x2, y1, z2), vtx(x2, y2, z2),
      // left
      vtx(x1, y1, z1), vtx(x1, y2, z2), vtx(x1, y2, z1),
      vtx(x1, y1, z1), vtx(x1, y1, z2), vtx(x1, y2, z2),
      // top
      vtx(x1, y2, z1), vtx(x2, y2, z2), vtx(x2, y2, z1),
      vtx(x1, y2, z1), vtx(x1, y2, z2), vtx(x2, y2, z2),
      // back
      vtx(x1, y1, z1), vtx(x2, y2, z1), vtx(x2, y1, z1),
      vtx(x1, y1, z1), vtx(x1, y2, z1), vtx(x2, y2, z1),
      // right
      vtx(x2, y1, z1), vtx(x2, y2, z2), vtx(x2, y1, z2),
      vtx(x2, y1, z1), vtx(x2, y2, z1), vtx(x2, y2, z2),
      // bottom
      vtx(x1, y1, z1), vtx(x2, y1, z2), vtx(x1, y1, z2),
      vtx(x1, y1, z1), vtx(x2, y1, z1), vtx(x2, y1, z2),
    ]
  }
}
