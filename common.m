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

function [a_x, a_y, r_x, r_y] = clip_ray(o_x, o_y, ir_x, ir_y)
  global atmos_radius;
  global planet_center_x;
  global planet_center_y;

  % normalize
  l = vec_len(ir_x, ir_y);
  l_x = ir_x ./ l;
  l_y = ir_y ./ l;

  % find the times where the ray intersects the atmosphere
  oc_x = o_x - planet_center_x;
  oc_y = o_y - planet_center_y;
  loc = dot(l_x, l_y, oc_x, oc_y);
  s = loc.*loc - dot(oc_x, oc_y, oc_x, oc_y) + atmos_radius.*atmos_radius;
  d1 = -sqrt(s) - loc;
  d2 =  sqrt(s) - loc;

  % if s < 0, we don't intersect. Set both times to 0.
  sign = s >= 0;
  d1 = d1 .* sign;
  d2 = d2 .* sign;

  % negative times mean either we're in the atmosphere, or it's entirely behind us.
  d1 = max(d1, 0);
  d2 = max(d2, 0);

  % reset [a] and [r] to be inside the atmosphere.
  a_x = o_x + l_x .* d1;
  a_y = o_y + l_y .* d1;
  r_x = l_x .* (d2 - d1);
  r_y = l_y .* (d2 - d1);
endfunction

function r = dot(x1, y1, x2, y2)
  r = x1.*x2 + y1.*y2;
endfunction

