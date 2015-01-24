use color::Color3;
use id_allocator::IdAllocator;
use nalgebra::{Pnt3, Vec3, normalize};
use ncollide::bounding_volume::AABB;
use state::EntityId;
use std::cmp::{partial_min, partial_max};
use std::rand::{Rng, SeedableRng, IsaacRng};
use terrain::LOD_QUALITY;
use terrain_block::{TerrainBlock, BLOCK_WIDTH};

/// Use one-octave perlin noise local maxima to place trees.
pub struct TreePlacer {
  seed: u32,
}

impl TreePlacer {
  pub fn new(seed: u32) -> TreePlacer {
    TreePlacer {
      seed: seed,
    }
  }

  fn rng_at(&self, center: &Pnt3<f32>, mut seed: Vec<u32>) -> IsaacRng {
    let center = *center * (LOD_QUALITY[0] as f32) / (BLOCK_WIDTH as f32);
    seed.push_all(&[self.seed, center.x as u32, center.z as u32]);
    SeedableRng::from_seed(seed.as_slice())
  }

  pub fn should_place_tree(&self, center: &Pnt3<f32>) -> bool {
    let mut rng = self.rng_at(center, vec!(0));
    rng.next_u32() > 0xFFF7FFFF
  }

  pub fn place_tree(
    &self,
    mut center: Pnt3<f32>,
    id_allocator: &mut IdAllocator<EntityId>,
    block: &mut TerrainBlock,
  ) {
    let normals = [
      normalize(&Vec3::new(-1.0, -1.0, -1.0)),
      normalize(&Vec3::new(-1.0, -1.0,  1.0)),
      normalize(&Vec3::new( 1.0, -1.0,  1.0)),
      normalize(&Vec3::new( 1.0, -1.0, -1.0)),
      normalize(&Vec3::new(-1.0,  1.0, -1.0)),
      normalize(&Vec3::new(-1.0,  1.0,  1.0)),
      normalize(&Vec3::new( 1.0,  1.0,  1.0)),
      normalize(&Vec3::new( 1.0,  1.0, -1.0)),
    ];

    let mut place_side =
      |&mut:
        corners: &[Pnt3<f32>],
        color: &Color3<f32>,
        idx1,
        idx2,
        idx3,
        idx4,
      | {
        let n1 = normals.get(idx1).unwrap();
        let n2 = normals.get(idx2).unwrap();
        let n3 = normals.get(idx3).unwrap();
        let n4 = normals.get(idx4).unwrap();

        let v1 = corners.get(idx1).unwrap();
        let v2 = corners.get(idx2).unwrap();
        let v3 = corners.get(idx3).unwrap();
        let v4 = corners.get(idx4).unwrap();

        block.vertex_coordinates.push_all(&[
          v1.x, v1.y, v1.z,
          v2.x, v2.y, v2.z,
          v4.x, v4.y, v4.z,

          v1.x, v1.y, v1.z,
          v4.x, v4.y, v4.z,
          v3.x, v3.y, v3.z,
        ]);

        block.normals.push_all(&[
          n1.x, n1.y, n1.z,
          n2.x, n2.y, n2.z,
          n4.x, n4.y, n4.z,

          n1.x, n1.y, n1.z,
          n4.x, n4.y, n4.z,
          n3.x, n3.y, n3.z,
        ]);

        block.colors.push_all(&[
          color.r, color.g, color.b,
          color.r, color.g, color.b,
        ]);

        let minx = partial_min(v1.x, v2.x).unwrap();
        let maxx = partial_max(v1.x, v2.x).unwrap();
        let minz = partial_min(v1.z, v2.z).unwrap();
        let maxz = partial_max(v1.z, v2.z).unwrap();

        let bounds = 
          AABB::new(
            Pnt3::new(minx, v1.y, minz),
            Pnt3::new(maxx, v3.y, maxz),
          );

        let id1 = id_allocator.allocate();
        let id2 = id_allocator.allocate();
        block.ids.push_all(&[id1, id2]);

        block.bounds.insert(id1, bounds.clone());
        block.bounds.insert(id2, bounds);
      };

    let mut rng = self.rng_at(&center, vec!(1));
    let mass = (rng.next_u32() as f32) / (0x10000 as f32) / (0x10000 as f32);
    let mass = 0.1 + mass * 0.9;
    let mass = partial_min(partial_max(0.0, mass).unwrap(), 1.0).unwrap();

    {
      let radius = mass * mass * 2.0;
      let height = mass * 16.0;
      let corners = [
        center + Vec3::new(-radius, 0.0, -radius),
        center + Vec3::new(-radius, 0.0,  radius),
        center + Vec3::new( radius, 0.0,  radius),
        center + Vec3::new( radius, 0.0, -radius),
        center + Vec3::new(-radius, height, -radius),
        center + Vec3::new(-radius, height,  radius),
        center + Vec3::new( radius, height,  radius),
        center + Vec3::new( radius, height, -radius),
      ];

      let color = Color3::of_rgb(0.4, 0.3, 0.1);
      place_side(&corners, &color, 0, 1, 4, 5);
      place_side(&corners, &color, 1, 2, 5, 6);
      place_side(&corners, &color, 2, 3, 6, 7);
      place_side(&corners, &color, 3, 0, 7, 4);

      center = center + Vec3::new(0.0, height, 0.0);
    }

    {
      let radius = mass * mass * 16.0;
      let height = mass * mass * 16.0;
      let corners = [
        center + Vec3::new(-radius, 0.0, -radius),
        center + Vec3::new(-radius, 0.0,  radius),
        center + Vec3::new( radius, 0.0,  radius),
        center + Vec3::new( radius, 0.0, -radius),
        center + Vec3::new(-radius, height, -radius),
        center + Vec3::new(-radius, height,  radius),
        center + Vec3::new( radius, height,  radius),
        center + Vec3::new( radius, height, -radius),
      ];

      let color = Color3::of_rgb(0.0, 0.4, 0.0);
      place_side(&corners, &color, 0, 1, 4, 5);
      place_side(&corners, &color, 1, 2, 5, 6);
      place_side(&corners, &color, 2, 3, 6, 7);
      place_side(&corners, &color, 3, 0, 7, 4);
      place_side(&corners, &color, 4, 5, 7, 6);
      place_side(&corners, &color, 1, 0, 2, 3);
    }
  }
}
