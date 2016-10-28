source "common.m";

function r = optical_depth(a_x, a_y, b_x, b_y)
  global atmos_radius;

  % "r" for "ray"
  r_x = b_x - a_x;
  r_y = b_y - a_y;

  [a_x, a_y, r_x, r_y] = clip_ray(a_x, a_y, b_x, b_y);

  samples = 20;
  l = vec_len(r_x, r_y) ./ samples;
  r_x = r_x ./ samples;
  r_y = r_y ./ samples;

  r = 0;
  for i = [1:samples]
    p_x = a_x + i .* r_x;
    p_y = a_y + i .* r_y;
    r = r + atmos_density(p_x, p_y) .* l;
  endfor
endfunction
