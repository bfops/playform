source "constants.m";

function r = vec_len(d_x, d_y)
  r = sqrt(d_x .* d_x + d_y .* d_y);
endfunction

function r = atmos_density (p_x, p_y)
  global planet_center_x;
  global planet_center_y;
  global planet_radius;
  global scale_height;
  d_x = p_x - planet_center_x;
  d_y = p_y - planet_center_y;
  dist_from_surface = vec_len(d_x, d_y) - planet_radius;
  r = exp(-dist_from_surface / scale_height);
endfunction

function [t1, t2] = line_sphere_intersect(o_x, o_y, l_x, l_y, c_x, c_y, r)
  % find the times where the ray intersects the atmosphere
  oc_x = o_x - c_x;
  oc_y = o_y - c_y;
  loc = dot(l_x, l_y, oc_x, oc_y);
  s = loc.*loc - dot(oc_x, oc_y, oc_x, oc_y) + r.*r;
  t1 = -sqrt(s) - loc;
  t2 =  sqrt(s) - loc;

  % if s < 0, we don't intersect. Set both times to 0.
  sign = s >= 0;
  t1 = t1 .* sign;
  t2 = t2 .* sign;

  % negative times mean either we're in the atmosphere, or it's entirely behind us.
  t1 = max(t1, 0);
  t2 = max(t2, 0);
endfunction

function [a_x, a_y, r_x, r_y] = clip_ray(o_x, o_y, d_x, d_y, c_x, c_y, r)
  % normalize
  l = vec_len(d_x, d_y);
  l_x = d_x ./ l;
  l_y = d_y ./ l;

  [t1, t2] = line_sphere_intersect(o_x, o_y, l_x, l_y, c_x, c_y, r);

  % reset [a] and [r] to be inside the atmosphere.
  a_x = o_x + l_x .* t1;
  a_y = o_y + l_y .* t1;
  r_x = l_x .* (t2 - t1);
  r_y = l_y .* (t2 - t1);
endfunction

function [a_x, a_y, r_x, r_y] = clip_atmos_ray(ia_x, ia_y, ir_x, ir_y)
  global atmos_radius;
  global planet_center_x;
  global planet_center_y;

  [a_x, a_y, r_x, r_y] = clip_ray(ia_x, ia_y, ir_x, ir_y, planet_center_x, planet_center_y, atmos_radius);
endfunction

function r = dot(x1, y1, x2, y2)
  r = x1.*x2 + y1.*y2;
endfunction

