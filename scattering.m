source "optical_depth.m";

function r = phase(cos_angle, g)
  c = cos_angle;
  g2 = g .* g;
  r = 3 * (1-g2) .* (1 + c.*c) ./ (2 * (2 + g2) .* realpow(1 + g2 - 2*g.*c, 3.0 / 2.0));
endfunction

function r = in_scatter(sun_angle, camera_x, camera_y, look_x, look_y, k, g)
  global planet_center_x;
  global planet_center_y;
  global planet_radius;
  global atmos_thickness;
  global sun_distance;
  sun_position_x = planet_center_x + cos(sun_angle) .* sun_distance;
  sun_position_y = planet_center_y + sin(sun_angle) .* sun_distance;

  samples = 100;
  l = atmos_thickness / samples;
  r = 0;
  for i = [1:samples]
    point_x = camera_x + look_x * i * l;
    point_y = camera_y + look_y * i * l;
    d1_x = sun_position_x - point_x;
    d1_y = sun_position_y - point_y;
    d2_x = camera_x - point_x;
    d2_y = camera_y - point_y;
    cos_angle = dot(d1_x, d1_y, d2_x, d2_y) ./ (vec_len(d1_x, d1_y) .* vec_len(d2_x, d2_y));
    out_scattered = 4*pi*k .* (optical_depth(camera_x, camera_y, point_x, point_y) + optical_depth(point_x, point_y, sun_position_x, sun_position_y));
    r += phase(cos_angle, g) .* atmos_density(point_x, point_y) .* exp(-out_scattered) * l;
  endfor
  r = k .* r;
endfunction
