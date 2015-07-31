use cgmath::{Point, Point3, Vector, Vector3};
use cgmath::Aabb3;
use std::cmp::{min, max};
use std::collections::hash_map;
use std::collections::HashMap;
use std::sync::Mutex;
use stopwatch::TimerSet;

use common::block_position::BlockPosition;
use common::entity::EntityId;
use common::id_allocator::IdAllocator;
use common::lod::LODIndex;
use common::terrain_block;
use common::terrain_block::{TerrainBlock, tri};

use field::Field;
use heightmap::HeightMap;
use voxel;
use voxel::{Fracu8, Fraci8, Voxel, SurfaceVoxel, Vertex, Normal};
use voxel_tree;
use voxel_tree::VoxelTree;

pub fn generate_voxel(
  timers: &TimerSet,
  heightmap: &HeightMap,
  voxel: &voxel::Bounds,
) -> Voxel
{
  timers.time("generate_voxel", || {
    let field_contains = |x, y, z| {
      heightmap.density_at(x, y, z) >= 0.0
    };

    let get_normal = |x, y, z| {
      heightmap.normal_at(0.01, x, y, z)
    };

    let size = voxel.size();
    let (x1, y1, z1) = (voxel.x as f32 * size, voxel.y as f32 * size, voxel.z as f32 * size);
    let delta = size;
    let (x2, y2, z2) = (x1 + delta, y1 + delta, z1 + delta);
    // corners[x][y][z]
    let corners = [
      [
        [ field_contains(x1, y1, z1), field_contains(x1, y1, z2) ],
        [ field_contains(x1, y2, z1), field_contains(x1, y2, z2) ],
      ],
      [
        [ field_contains(x2, y1, z1), field_contains(x2, y1, z2) ],
        [ field_contains(x2, y2, z1), field_contains(x2, y2, z2) ],
      ],
    ];

    let corner;
    let mut any_inside = false;
    let mut all_inside = true;

    {
      let mut get_corner = |x1:usize, y1:usize, z1:usize| {
        let corner = corners[x1][y1][z1];
        any_inside = any_inside || corner;
        all_inside = all_inside && corner;
        corner
      };

      corner = get_corner(0,0,0);
      let _ = get_corner(0,0,1);
      let _ = get_corner(0,1,0);
      let _ = get_corner(0,1,1);
      let _ = get_corner(1,0,0);
      let _ = get_corner(1,0,1);
      let _ = get_corner(1,1,0);
      let _ = get_corner(1,1,1);
    }

    let all_corners_same = any_inside == all_inside;
    if all_corners_same {
      return Voxel::Volume(all_inside)
    }

    let mut vertex: Vector3<u32> = Vector3::new(0, 0, 0);
    let mut n = 0;
    for (&x, corners) in [0, 0xFF].iter().zip(corners.iter()) {
    for (&y, corners) in [0, 0xFF].iter().zip(corners.iter()) {
    for (&z, &corner) in [0, 0xFF].iter().zip(corners.iter()) {
      if corner {
        vertex.add_self_v(&Vector3::new(x, y, z));
        n += 1;
      }
    }}}

    {
      // Sample in extra areas to help weight the vertex to the appropriate place.
      let mut sample_extra = |lg_s: u8| {
        let fs = 1.0 / ((1 << lg_s) as f32);
        let mfs = 1.0 - fs;
        let s = 0x100 >> lg_s;
        let ms = 0x100 - s;
        for &(wx, x) in
          [((voxel.x as f32 + fs) * size, s), ((voxel.x as f32 + mfs) * size, ms)].iter() {
        for &(wy, y) in
          [((voxel.y as f32 + fs) * size, s), ((voxel.y as f32 + mfs) * size, ms)].iter() {
        for &(wz, z) in
          [((voxel.z as f32 + fs) * size, s), ((voxel.z as f32 + mfs) * size, ms)].iter() {
          if field_contains(wx, wy, wz) {
            vertex.add_self_v(&Vector3::new(x, y, z).mul_s(lg_s as u32));
            n += lg_s as u32;
          }
        }}}
      };

      sample_extra(2);
    }

    let vertex = vertex.div_s(n);
    let vertex =
      Vertex {
        x: Fracu8::of(vertex.x as u8),
        y: Fracu8::of(vertex.y as u8),
        z: Fracu8::of(vertex.z as u8),
      };

    let normal;
    {
      // Okay, this is silly to have right after we construct the vertex.
      let vertex = vertex.to_world_vertex(voxel);
      normal = get_normal(vertex.x, vertex.y, vertex.z).mul_s(127.0);
    }

    // Okay, so we scale the normal by 127, and use 127 to represent 1.0.
    // Then we store it in a `Fraci8`, which scales by 128 and represents a
    // fraction in [0,1). That seems wrong, but this is normal data, so scaling
    // doesn't matter. Sketch factor is over 9000, but it's not wrong.

    let normal = Vector3::new(normal.x as i32, normal.y as i32, normal.z as i32);
    let normal =
      Vector3::new(
        max(-127, min(127, normal.x)) as i8,
        max(-127, min(127, normal.y)) as i8,
        max(-127, min(127, normal.z)) as i8,
      );

    let normal =
      Normal {
        x: Fraci8::of(normal.x),
        y: Fraci8::of(normal.y),
        z: Fraci8::of(normal.z),
      };

    Voxel::Surface(SurfaceVoxel {
      inner_vertex: vertex,
      normal: normal,
      corner_inside_surface: corner,
    })
  })
}

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

/// Generate a `TerrainBlock` based on a given position in a `VoxelTree`.
/// Any necessary voxels will be generated.
pub fn generate_block(
  timers: &TimerSet,
  id_allocator: &Mutex<IdAllocator<EntityId>>,
  heightmap: &HeightMap,
  voxels: &mut VoxelTree,
  position: &BlockPosition,
  lod_index: LODIndex,
) -> TerrainBlock {
  timers.time("update.generate_block", || {
    let mut block = TerrainBlock::empty();

    let lateral_samples = terrain_block::EDGE_SAMPLES[lod_index.0 as usize] as i32;
    let lg_size = terrain_block::LG_SAMPLE_SIZE[lod_index.0 as usize] as i16;

    let bounds_at = |v: &Point3<i32>| {
      voxel::Bounds::new(v.x, v.y, v.z, lg_size)
    };

    let mut get_voxel = |bounds: &voxel::Bounds| {
      let branch = voxels.get_mut_or_create(bounds);
      let r;
      match branch {
        &mut voxel_tree::TreeBody::Leaf(v) => r = v,
        &mut voxel_tree::TreeBody::Empty => {
          r = generate_voxel(timers, heightmap, bounds);
          *branch = voxel_tree::TreeBody::Leaf(r);
        },
        &mut voxel_tree::TreeBody::Branch(_) => {
          // Overwrite existing for now.
          // TODO: Don't do ^that.
          r = generate_voxel(timers, heightmap, bounds);
          *branch = voxel_tree::TreeBody::Leaf(r);
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
        Voxel::Surface(v) => voxel = v,
        _ => continue,
      }
      let index = coords.len();
      let vertex = voxel.inner_vertex.to_world_vertex(&bounds);
      let normal = voxel.normal.to_world_normal();
      coords.push(vertex);
      normals.push(normal);
      indices.insert(voxel_position, index);

      let mut edge = |
        d_neighbor, // Vector to the neighbor to make an edge toward.
        d1, d2,     // Vector to the voxels adjacent to the edge.
      | {
        let neighbor_inside_surface;
        match get_voxel(&bounds_at(&voxel_position.add_v(&d_neighbor))) {
          Voxel::Surface(v) => neighbor_inside_surface = v.corner_inside_surface,
          Voxel::Volume(inside) => neighbor_inside_surface = inside,
        }
        if voxel.corner_inside_surface == neighbor_inside_surface {
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
                  Voxel::Surface(voxel) => {
                    let i = coords.len();
                    let vertex = voxel.inner_vertex.to_world_vertex(&bounds);
                    let normal = voxel.normal.to_world_normal();
                    coords.push(vertex);
                    normals.push(normal);
                    entry.insert(i);
                    (vertex, normal, i)
                  },
                  voxel => {
                    panic!("Unexpected neighbor {:?}", voxel)
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

        if voxel.corner_inside_surface {
          // The polys are visible from positive infinity.
          polys.push([i2, i1, i_center]);
          polys.push([i3, i2, i_center]);
          polys.push([i4, i3, i_center]);
          polys.push([i1, i4, i_center]);
        } else {
          // The polys are visible from negative infinity.
          polys.push([i1, i2, i_center]);
          polys.push([i2, i3, i_center]);
          polys.push([i3, i4, i_center]);
          polys.push([i4, i1, i_center]);
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

    for poly in polys.iter() {
      block.vertex_coordinates.push(tri(coords[poly[0]], coords[poly[1]], coords[poly[2]]));
      block.normals.push(tri(normals[poly[0]], normals[poly[1]], normals[poly[2]]));

      let id = id_allocator.lock().unwrap().allocate();
      block.ids.push(id);
      block.bounds.push((id, make_bounds(&coords[poly[0]], &coords[poly[1]], &coords[poly[2]])));
    }

    block
  })
}
