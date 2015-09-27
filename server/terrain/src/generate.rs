use cgmath::{Aabb3, Point, Point3, Vector, Vector3};
use std::collections::hash_map;
use std::collections::HashMap;
use std::sync::Mutex;
use stopwatch;

use common::block_position::BlockPosition;
use common::entity::EntityId;
use common::id_allocator::IdAllocator;
use common::lod::LODIndex;
use common::terrain_block;
use common::terrain_block::{TerrainBlock, tri};

use voxel;
use voxel_base;

#[inline]
fn partial_min<I, It>(mut it: It) -> Option<I>
where
  I: PartialOrd,
  It: Iterator<Item=I>,
{
  match it.next() {
    None => None,
    Some(mut best) => {
      for thing in it {
        if thing < best {
          best = thing;
        }
      }
      Some(best)
    },
  }
}

#[inline]
fn partial_max<I, It>(mut it: It) -> Option<I>
where
  I: PartialOrd,
  It: Iterator<Item=I>,
{
  match it.next() {
    None => None,
    Some(mut best) => {
      for thing in it {
        if thing > best {
          best = thing;
        }
      }
      Some(best)
    },
  }
}

fn make_bounds(
  v1: &Point3<f32>,
  v2: &Point3<f32>,
  v3: &Point3<f32>,
) -> Aabb3<f32> {
  let minx = *partial_min([v1.x, v2.x, v3.x].iter()).unwrap();
  let maxx = *partial_max([v1.x, v2.x, v3.x].iter()).unwrap();

  let miny = *partial_min([v1.y, v2.y, v3.y].iter()).unwrap();
  let maxy = *partial_max([v1.y, v2.y, v3.y].iter()).unwrap();

  let minz = *partial_min([v1.z, v2.z, v3.z].iter()).unwrap();
  let maxz = *partial_max([v1.z, v2.z, v3.z].iter()).unwrap();

  Aabb3::new(
    // TODO: Remove this - 1.0. It's a temporary hack until voxel collisions work,
    // to avoid zero-height Aabb3s.
    Point3::new(minx, miny - 1.0, minz),
    Point3::new(maxx, maxy, maxz),
  )
}

/// Generate a `TerrainBlock` based on a given position in a `voxel::tree::T`.
/// Any necessary voxels will be generated.
pub fn generate_block<Mosaic>(
  id_allocator: &Mutex<IdAllocator<EntityId>>,
  mosaic: &Mosaic,
  voxels: &mut voxel::tree::T,
  position: &BlockPosition,
  lod_index: LODIndex,
) -> TerrainBlock
  where Mosaic: voxel_base::mosaic::T<Material=voxel::Material>,
{
  stopwatch::time("update.generate_block", || {
    let mut block = TerrainBlock::empty();

    let lateral_samples = terrain_block::EDGE_SAMPLES[lod_index.0 as usize] as i32;
    let lg_size = terrain_block::LG_SAMPLE_SIZE[lod_index.0 as usize] as i16;

    let bounds_at = |v: &Point3<i32>| {
      voxel_base::bounds::new(v.x, v.y, v.z, lg_size)
    };

    let mut get_voxel = |bounds: &voxel_base::bounds::T| {
      let branch = voxels.get_mut_or_create(bounds);
      let r;
      match branch {
        &mut voxel::tree::Empty => {
          r = voxel::unwrap(voxel::of_field(mosaic, bounds));
          *branch =
            voxel::tree::Branch {
              data: Some(r),
              branches: Box::new(voxel::tree::Branches::empty()),
            };
        },
        &mut voxel::tree::Branch { ref mut data, branches: _ }  => {
          match data {
            &mut None => {
              r = voxel::unwrap(voxel::of_field(mosaic, bounds));
              *data = Some(r);
            },
            &mut Some(ref data) => {
              r = *data;
            },
          }
        },
      };
      r
    };

    let lg_ratio = terrain_block::LG_WIDTH - lg_size;
    let block_position = position.as_pnt();
    let voxel_position =
      Point3::new(
        block_position.x << lg_ratio,
        block_position.y << lg_ratio,
        block_position.z << lg_ratio,
      );

    let mut coords = Vec::new();
    let mut normals = Vec::new();
    let mut indices = HashMap::new();
    let mut polys = Vec::new();

    for dx in 0..lateral_samples {
    for dy in 0..lateral_samples {
    for dz in 0..lateral_samples {
      let voxel_position = voxel_position.add_v(&Vector3::new(dx, dy, dz));
      let voxel;
      let bounds = bounds_at(&voxel_position);
      match get_voxel(&bounds) {
        voxel::T::Surface(v) => voxel = v,
        _ => continue,
      }

      debug!("Generate from {:?}, contents {:?}", bounds, voxel);
      let index = coords.len();
      let vertex = voxel.surface_vertex.to_world_vertex(&bounds);
      let normal = voxel.normal.to_float_normal();
      coords.push(vertex);
      normals.push(normal);
      indices.insert(voxel_position, index);

      let mut edge = |
        d_neighbor, // Vector to the neighbor to make an edge toward.
        d1, d2,     // Vector to the voxels adjacent to the edge.
      | {
        let neighbor_position = voxel_position.add_v(&d_neighbor);
        let neighbor_material;
        match get_voxel(&bounds_at(&neighbor_position)) {
          voxel::T::Surface(v) => neighbor_material = v.corner,
          voxel::T::Volume(material) => neighbor_material = material,
        }
        debug!("edge from {:?} to {:?}", voxel_position, neighbor_position);
        debug!("neighbor is {:?}", neighbor_material);
        let empty = voxel::Material::Empty;
        if (voxel.corner == empty) == (neighbor_material == empty) {
          // This edge doesn't cross the surface, and doesn't generate polys.

          return
        }

        let v1; let v2; let v3; let v4;
        let n1; let n2; let n3; let n4;
        let i1; let i2; let i3; let i4;

        {
          let mut voxel_index = |position: &Point3<i32>| {
            match indices.entry(*position) {
              hash_map::Entry::Occupied(entry) => {
                let i = *entry.get();
                (coords[i], normals[i], i)
              },
              hash_map::Entry::Vacant(entry) => {
                let bounds = bounds_at(position);
                match get_voxel(&bounds) {
                  voxel::T::Surface(voxel) => {
                    let i = coords.len();
                    let vertex = voxel.surface_vertex.to_world_vertex(&bounds);
                    let normal = voxel.normal.to_float_normal();
                    coords.push(vertex);
                    normals.push(normal);
                    entry.insert(i);
                    (vertex, normal, i)
                  },
                  voxel => {
                    panic!("Unexpected neighbor {:?} at {:?}", voxel, bounds)
                  }
                }
              },
            }
          };

          let (tv1, tn1, ti1) = voxel_index(&voxel_position.add_v(&d1).add_v(&d2));
          let (tv2, tn2, ti2) = voxel_index(&voxel_position.add_v(&d1));
          let (tv3, tn3, ti3) = (vertex, normal, index);
          let (tv4, tn4, ti4) = voxel_index(&voxel_position.add_v(&d2));

          v1 = tv1; v2 = tv2; v3 = tv3; v4 = tv4;
          n1 = tn1; n2 = tn2; n3 = tn3; n4 = tn4;
          i1 = ti1; i2 = ti2; i3 = ti3; i4 = ti4;
        }

        // Put a vertex at the average of the vertices.
        let v_center =
          v1.add_v(&v2.to_vec()).add_v(&v3.to_vec()).add_v(&v4.to_vec()).div_s(4.0);
        let n_center =
          n1.add_v(&n2).add_v(&n3).add_v(&n4).div_s(4.0);

        let i_center = coords.len();
        coords.push(v_center);
        normals.push(n_center);

        if voxel.corner == voxel::Material::Empty {
          // The polys are visible from negative infinity.
          polys.push([i1, i2, i_center]);
          polys.push([i2, i3, i_center]);
          polys.push([i3, i4, i_center]);
          polys.push([i4, i1, i_center]);
          block.materials.push_all(&[neighbor_material as i32; 4]);
        } else {
          // The polys are visible from positive infinity.
          polys.push([i2, i1, i_center]);
          polys.push([i3, i2, i_center]);
          polys.push([i4, i3, i_center]);
          polys.push([i1, i4, i_center]);
          block.materials.push_all(&[voxel.corner as i32; 4]);
        }
      };

      edge(
        Vector3::new(1, 0, 0),
        Vector3::new(0, -1, 0),
        Vector3::new(0, 0, -1),
      );

      edge(
        Vector3::new(0, 1, 0),
        Vector3::new(0, 0, -1),
        Vector3::new(-1, 0, 0),
      );

      edge(
        Vector3::new(0, 0, 1),
        Vector3::new(-1, 0, 0),
        Vector3::new(0, -1, 0),
      );
    }}}

    for poly in &polys {
      block.vertex_coordinates.push(tri(coords[poly[0]], coords[poly[1]], coords[poly[2]]));
      block.normals.push(tri(normals[poly[0]], normals[poly[1]], normals[poly[2]]));

      let id = id_allocator.lock().unwrap().allocate();
      block.ids.push(id);
      block.bounds.push((id, make_bounds(&coords[poly[0]], &coords[poly[1]], &coords[poly[2]])));
    }

    block
  })
}
