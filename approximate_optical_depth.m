source "approximate_optical_depth_poly.m"

global optical_depth_poly = approximate_optical_depth_poly();

function r = approximate_optical_depth(a_x, a_y, b_x, b_y)
  global planet_center_x;
  global planet_center_y;
  global optical_depth_poly;

  da_x = a_x - planet_center_x;
  da_y = a_y - planet_center_y;
  db_x = b_x - planet_center_x;
  db_y = b_y - planet_center_y;
  d_x = b_x - a_x;
  d_y = b_y - a_y;
  d = vec_len(d_x, d_y);
  d_x = d_x ./ d;
  d_y = d_y ./ d;
  a_t = pi - acos(dot(da_x, da_y, d_x, d_y) ./ vec_len(da_x, da_y));
  b_t = pi - acos(dot(db_x, db_y, d_x, d_y) ./ vec_len(db_x, db_y));

  r = atmos_density(a_x, a_y) .* polyval(optical_depth_poly, a_t) - atmos_density(b_x, b_y) .* polyval(optical_depth_poly, b_t);
end
